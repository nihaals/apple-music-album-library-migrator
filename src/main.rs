mod apple_music;
mod matching;

use anyhow::{Result, ensure};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

use crate::apple_music::custom_types;

#[derive(Parser)]
#[command(version, author, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Migrates library status of songs from one album to another
    Migrate {
        /// Apple Music developer token JWT
        #[arg(short = 'D', long)]
        developer_token: String,

        /// Origin header value
        #[arg(short = 'O', long = "origin")]
        origin_header: Option<String>,

        /// Apple Music User Token
        #[arg(short = 'U', long)]
        user_token: String,

        /// Apple Music API host
        #[arg(short = 'H', long)]
        host: Host,

        /// Apple Music catalog storefront (e.g. `us`)
        #[arg(short = 'S', long)]
        storefront: String,

        /// Print the matched tracks from the source and destination and do not make any changes
        #[arg(long)]
        dry_run: bool,

        /// The library ID (starts with `l.`) of the album that has songs added to the library
        source_album_library_id: String,

        /// The catalog ID (numeric) of the album that will have songs added to the library
        destination_album_catalog_id: String,
    },

    /// Generate shell completions
    Completions {
        /// The shell to generate the completions for
        #[arg(value_enum)]
        shell: clap_complete_command::Shell,
    },
}

#[derive(ValueEnum, Clone, Copy)]
enum Host {
    AmpApi,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Migrate {
            developer_token,
            origin_header,
            user_token,
            host: _,
            storefront,
            dry_run,
            source_album_library_id,
            destination_album_catalog_id,
        } => {
            ensure!(
                apple_music::validate_developer_token(&developer_token),
                "invalid developer token",
            );
            ensure!(
                apple_music::validate_storefront(&storefront),
                "invalid storefront",
            );
            ensure!(
                apple_music::validate_library_album_id(&source_album_library_id),
                "invalid source album library ID",
            );
            ensure!(
                apple_music::validate_catalog_id(&destination_album_catalog_id),
                "invalid destination album catalog ID",
            );

            let client =
                apple_music::Client::new(&developer_token, origin_header, user_token, storefront)?;
            let source_album = {
                let library_album = client.get_library_album(&source_album_library_id).await?;
                ensure!(library_album.library_id()? == source_album_library_id);
                let catalog_album = client
                    .get_catalog_album(library_album.catalog_id()?)
                    .await?;
                let album: custom_types::Album<custom_types::TrackNoLibrary> =
                    catalog_album.try_into()?;
                album.with_library_info(&library_album)?
            };
            let destination_album: custom_types::Album<custom_types::TrackNoLibrary> = client
                .get_catalog_album(&destination_album_catalog_id)
                .await?
                .try_into()?;
            ensure!(destination_album.catalog_id == destination_album_catalog_id);
            ensure!(
                source_album.catalog_id != destination_album.catalog_id,
                "source and destination albums are the same",
            );

            let matches = matching::match_tracks(&source_album, &destination_album)?;

            if dry_run {
                println!(
                    "Source: \"{}\" by {} ({}, {} tracks)",
                    source_album.name,
                    source_album.artist_name,
                    source_album.release_date,
                    source_album.tracks.len(),
                );
                println!(
                    "Destination: \"{}\" by {} ({}, {} tracks)",
                    destination_album.name,
                    destination_album.artist_name,
                    destination_album.release_date,
                    destination_album.tracks.len(),
                );
                println!();

                let mut matched = Vec::new();
                let mut unmatched = Vec::new();

                for result in &matches {
                    match result {
                        matching::TrackMatchResult::Match {
                            source,
                            destination,
                        } => {
                            if source.library_id.is_none() {
                                continue;
                            }
                            let src_num = source_album
                                .tracks
                                .iter()
                                .position(|t| t.catalog_id == source.catalog_id)
                                .unwrap()
                                + 1;
                            let dst_num = destination_album
                                .tracks
                                .iter()
                                .position(|t| t.catalog_id == destination.catalog_id)
                                .unwrap()
                                + 1;
                            matched.push((src_num, source, dst_num, destination));
                        }
                        matching::TrackMatchResult::NoMatch { source } => {
                            if source.library_id.is_none() {
                                continue;
                            }
                            let src_num = source_album
                                .tracks
                                .iter()
                                .position(|t| t.catalog_id == source.catalog_id)
                                .unwrap()
                                + 1;
                            unmatched.push((src_num, *source));
                        }
                    }
                }

                if !matched.is_empty() {
                    println!("Matched tracks:");
                    for (src_num, source, dst_num, destination) in &matched {
                        let both_explicit = source.is_explicit && destination.is_explicit;
                        let src_explicit = if source.is_explicit && !both_explicit {
                            " [E]"
                        } else {
                            ""
                        };
                        let dst_explicit = if destination.is_explicit && !both_explicit {
                            " [E]"
                        } else {
                            ""
                        };
                        if source.name == destination.name
                            && source.artist_name == destination.artist_name
                        {
                            println!(
                                "  #{src_num}{src_explicit} \u{2192} #{dst_num}{dst_explicit} {}",
                                source.name,
                            );
                        } else {
                            println!(
                                "  #{src_num} {}{src_explicit} \u{2192} #{dst_num} {}{dst_explicit}",
                                source.name, destination.name,
                            );
                        }
                    }
                }

                if !unmatched.is_empty() {
                    if !matched.is_empty() {
                        println!();
                    }
                    println!("Unmatched tracks (in library, no match in destination):");
                    for (src_num, source) in &unmatched {
                        let src_explicit = if source.is_explicit { " [E]" } else { "" };
                        println!("  #{src_num} {}{src_explicit}", source.name);
                    }
                }

                if matched.is_empty() && unmatched.is_empty() {
                    println!("No tracks in the library to migrate.");
                }

                return Ok(());
            }

            let songs_to_add: Vec<&str> = matches
                .iter()
                .filter_map(|result| match result {
                    matching::TrackMatchResult::Match {
                        source,
                        destination,
                    } if source.library_id.is_some() => Some(destination.catalog_id.as_str()),
                    _ => None,
                })
                .collect();

            ensure!(!songs_to_add.is_empty(), "no tracks to migrate");

            println!("Before:");
            for (i, track) in source_album.tracks.iter().enumerate() {
                let in_library = if track.library_id.is_some() {
                    " [in library]"
                } else {
                    ""
                };
                println!("  #{} {}{in_library}", i + 1, track.name);
            }

            client
                .remove_album_from_library(&source_album_library_id)
                .await?;

            client.add_songs_to_library(&songs_to_add).await?;

            println!();
            println!("After:");
            for (i, track) in destination_album.tracks.iter().enumerate() {
                let added = if songs_to_add.contains(&track.catalog_id.as_str()) {
                    " [added]"
                } else {
                    ""
                };
                println!("  #{} {}{added}", i + 1, track.name);
            }
        }
        Commands::Completions { shell } => {
            shell.generate(&mut Cli::command(), &mut std::io::stdout());
        }
    }
    Ok(())
}
