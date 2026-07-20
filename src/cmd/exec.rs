//! Exec command
use crate::{Error, Result};
use clap::Args;

/// Exec command arguments
#[derive(Args)]
#[command(group = clap::ArgGroup::new("question-id").args(&["id", "daily"]).required(true))]
pub struct ExecArgs {
    /// Question id
    #[arg(value_parser = clap::value_parser!(i32))]
    pub id: Option<i32>,

    /// Submit today's daily challenge
    #[arg(short = 'd', long)]
    pub daily: bool,
}

impl ExecArgs {
    /// `exec` handler
    pub async fn run(&self) -> Result<()> {
        use crate::cache::{Cache, Run};
        use crate::helper::append_failed_case;
        use colored::Colorize;

        let cache = Cache::new()?;

        let daily_id = if self.daily {
            Some(cache.get_daily_problem_id().await?)
        } else {
            None
        };

        let id = self.id.or(daily_id).ok_or(Error::NoneError)?;

        let res = cache.exec_problem(id, Run::Submit, None).await?;

        println!("{}", res);

        // Optionally remember the failing submit case for local re-test.
        let conf = &cache.0.conf;
        if conf.code.save_failed_cases {
            if let Some(case) = res.failed_submit_case() {
                let problem = cache.get_problem(id)?;
                let max = conf.code.max_saved_cases.max(1);
                match append_failed_case(&problem, case, max) {
                    Ok(true) => {
                        if let Ok(path) = crate::helper::test_cases_path(&problem) {
                            println!(
                                "\n{} saved failed case → {}",
                                "hint:".green().bold(),
                                path.dimmed()
                            );
                        }
                    }
                    Ok(false) => {
                        println!(
                            "\n{} failed case already in tests.dat, skipped",
                            "hint:".dimmed()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "\n{} could not save failed case: {}",
                            "warn:".yellow().bold(),
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
