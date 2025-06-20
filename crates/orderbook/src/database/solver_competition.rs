use {
    super::Postgres,
    crate::solver_competition::{Identifier, LoadSolverCompetitionError, SolverCompetitionStoring},
    anyhow::{Context, Result},
    database::{self, byte_array::ByteArray},
    model::{
        auction::AuctionId,
        solver_competition::{SolverCompetitionAPI, SolverCompetitionDB},
    },
    primitive_types::H256,
    sqlx::types::JsonValue,
};

fn deserialize_solver_competition(
    json: JsonValue,
    auction_id: AuctionId,
    transaction_hashes: Vec<H256>,
) -> Result<SolverCompetitionAPI, LoadSolverCompetitionError> {
    let common: SolverCompetitionDB =
        serde_json::from_value(json).context("deserialize SolverCompetitionDB")?;
    Ok(SolverCompetitionAPI {
        auction_id,
        transaction_hashes,
        common,
    })
}

async fn attach_solution_hashes(
    ex: &mut sqlx::PgConnection,
    auction_id: AuctionId,
    competition: &mut SolverCompetitionAPI,
) -> Result<()> {
    for solution in &mut competition.common.solutions {
        let address: database::Address = ByteArray(solution.solver_address.0);
        if let Some(hash) = database::settlements::find_settlement_transaction(ex, auction_id, address).await? {
            solution.transaction_hash = Some(H256(hash.0));
        }
    }
    Ok(())
}

#[async_trait::async_trait]
impl SolverCompetitionStoring for Postgres {
    async fn load_competition(
        &self,
        id: Identifier,
    ) -> Result<SolverCompetitionAPI, LoadSolverCompetitionError> {
        let _timer = super::Metrics::get()
            .database_queries
            .with_label_values(&["load_solver_competition"])
            .start_timer();

        let mut ex = self.pool.acquire().await.map_err(anyhow::Error::from)?;
        let mut competition = match id {
            Identifier::Id(id) => {
                let row = database::solver_competition::load_by_id(&mut ex, id)
                    .await
                    .context("solver_competition::load_by_id")?;
                row.map(|row| {
                    deserialize_solver_competition(
                        row.json,
                        row.id,
                        row.tx_hashes.iter().map(|hash| H256(hash.0)).collect(),
                    )
                })
            }
            Identifier::Transaction(hash) => {
                let row = database::solver_competition::load_by_tx_hash(&mut ex, &ByteArray(hash.0))
                    .await
                    .context("solver_competition::load_by_tx_hash")?;
                row.map(|row| {
                    deserialize_solver_competition(
                        row.json,
                        row.id,
                        row.tx_hashes.iter().map(|hash| H256(hash.0)).collect(),
                    )
                })
            }
        }
        .ok_or(LoadSolverCompetitionError::NotFound)?;
        attach_solution_hashes(&mut ex, competition.auction_id, &mut competition).await?;
        Ok(competition)
    }

    async fn load_latest_competition(
        &self,
    ) -> Result<SolverCompetitionAPI, LoadSolverCompetitionError> {
        let _timer = super::Metrics::get()
            .database_queries
            .with_label_values(&["load_latest_solver_competition"])
            .start_timer();

        let mut ex = self.pool.acquire().await.map_err(anyhow::Error::from)?;
        let row = database::solver_competition::load_latest_competition(&mut ex)
            .await
            .context("solver_competition::load_latest_competition")?;
        let mut competition = row
            .map(|row| {
                deserialize_solver_competition(
                    row.json,
                    row.id,
                    row.tx_hashes.iter().map(|hash| H256(hash.0)).collect(),
                )
            })
            .ok_or(LoadSolverCompetitionError::NotFound)?;
        attach_solution_hashes(&mut ex, competition.auction_id, &mut competition).await?;
        Ok(competition)
    }

    async fn load_latest_competitions(
        &self,
        latest_competitions_count: u32,
    ) -> Result<Vec<SolverCompetitionAPI>, LoadSolverCompetitionError> {
        let _timer = super::Metrics::get()
            .database_queries
            .with_label_values(&["load_latest_competitions"])
            .start_timer();

        let mut ex = self.pool.acquire().await.map_err(anyhow::Error::from)?;

        let mut latest_competitions = database::solver_competition::load_latest_competitions(
            &mut ex,
            latest_competitions_count,
        )
        .await
        .context("solver_competition::load_latest_competitions")?
        .into_iter()
        .map(|row| {
            deserialize_solver_competition(
                row.json,
                row.id,
                row.tx_hashes.iter().map(|hash| H256(hash.0)).collect(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

        for competition in &mut latest_competitions {
            attach_solution_hashes(&mut ex, competition.auction_id, competition).await?;
        }

        Ok(latest_competitions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn not_found_error() {
        let db = Postgres::try_new("postgresql://").unwrap();
        database::clear_DANGER(&db.pool).await.unwrap();

        let result = db
            .load_competition(Identifier::Transaction(Default::default()))
            .await
            .unwrap_err();
        assert!(matches!(result, LoadSolverCompetitionError::NotFound));
    }
}
