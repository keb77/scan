use clap::{Parser as ClapParser, Subcommand};
use log::info;
use scan_fmt_xml::{Parser, Sc2CsVisitor};
use std::{error::Error, path::PathBuf};

/// SCAN (StoChastic ANalyzer)
/// is a statistical model checker based on channel systems
#[derive(ClapParser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Action to perform on model
    #[command(subcommand)]
    command: Commands,

    /// Select model XML file
    model: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify model
    Verify {
        /// lists test values
        #[arg(short, long)]
        runs: usize,
    },
    /// Parse and validate model XML file
    Validate,
    /// Executes model once
    Execute,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("SCAN starting up");

    let cli = Cli::parse();
    info!("cli arguments parsed");

    match &cli.command {
        Commands::Verify { runs: _ } => {
            println!("Verifying model - NOT YET IMPLEMENTED");

            // info!("parsing model");
            // let model = Parser::parse(cli.model.to_owned())?;
            // let cs = Sc2CsVisitor::visit(model)?;

            // for run in 0..*runs {
            //     info!("verify model, run {run}");
            //     let mut model = model.clone();
            //     while let Some((pg_id, action, post)) = model.possible_transitions().first() {
            //         model
            //             .transition(*pg_id, *action, *post)
            //             .expect("transition possible");
            //         println!("{model:#?}");
            //     }
            //     info!("model verified");
            // }
        }
        Commands::Validate => {
            println!("Validating model");

            // info!("creating reader from file {0}", cli.model.display());
            // let mut reader = Reader::from_file(cli.model)?;

            info!("parsing model");
            // let model = Parser::parse(&mut reader)?;
            let model = Parser::parse(cli.model.to_owned())?;
            println!("{model:#?}");

            info!("building CS representation");
            let cs = Sc2CsVisitor::visit(model)?;
            println!("{cs:#?}");

            println!("Model successfully validated");
        }
        Commands::Execute => {
            // info!("creating reader from file {0}", cli.model.display());
            // let mut reader = Reader::from_file(cli.model)?;

            info!("parsing model");
            // let model = Parser::parse(&mut reader)?;
            let parser = Parser::parse(cli.model.to_owned())?;
            info!("parsing successful");

            info!("building CS representation");
            let mut model = Sc2CsVisitor::visit(parser)?;
            info!("building successful");

            println!("Executing model");
            while let Some((pg_id, action, destination)) =
                model.cs.possible_transitions().first().cloned()
            {
                let pg = model
                    .fsm_names
                    .get(&pg_id)
                    .cloned()
                    .unwrap_or_else(|| format!("{pg_id:?}"));
                println!("transition PG {pg} by {action:?} to {destination:?}");
                model.cs.transition(pg_id, action, destination)?;
            }

            println!("Model run to termination");
        }
    }

    info!("SCAN terminating");
    Ok(())
}
