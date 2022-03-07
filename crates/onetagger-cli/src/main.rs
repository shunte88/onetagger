#[macro_use] extern crate log;
#[macro_use] extern crate onetagger_shared;

use std::error::Error;
use std::fs::File;
use clap::{Parser, Subcommand};
use onetagger_platforms::spotify::Spotify;
use onetagger_shared::{VERSION, COMMIT};
use onetagger_autotag::audiofeatures::{AudioFeaturesConfig, AudioFeatures};
use onetagger_autotag::{Tagger, TaggerConfigExt};
use onetagger_tagger::TaggerConfig;


fn main() {
    let cli = Cli::parse();

    // Default configs
    if cli.autotagger_config {
        let config = serde_json::to_string_pretty(&TaggerConfig::custom_default()).expect("Failed serializing default config!");
        println!("{config}");
        return;
    }
    if cli.audiofeatures_config {
        let config = serde_json::to_string_pretty(&AudioFeaturesConfig::default()).expect("Failed serializing config!");
        println!("{config}");
        return;
    }

    if cli.action.is_none() {
        println!("No action. Use onetagger-cli --help to get print help.");
        return;
    }

    // Setup logging
    onetagger_shared::setup();
    info!("\n\nStarting OneTagger v{VERSION} Commit: {COMMIT} OS: {}\n\n", std::env::consts::OS);


    let action = cli.action.unwrap();
    match &action {
        Actions::Autotagger { path, .. } => {
            let config = action.get_at_config().expect("Failed loading config file!");
            let files = Tagger::get_file_list(&path, config.include_subfolders);
            let rx = Tagger::tag_files(&config, files);
            let start = timestamp!();
            for status in rx {
                debug!("{status:?}");
            }
            info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
        },
        Actions::Audiofeatures { path, config, client_id, client_secret, no_subfolders } => {
            let file = File::open(config).expect("Failed reading config file!");
            let config: AudioFeaturesConfig = serde_json::from_reader(&file).expect("Failed parsing config file!");
            // Cli subfolders override
            let mut subfolders = config.include_subfolders;
            if *no_subfolders {
                subfolders = false;
            }
            // Auth spotify
            let spotify = Spotify::try_cached_token(client_id, client_secret)
                .expect("Spotify unauthorized, please run the authorize-spotify option or login to Spotify in UI at least once!");
            
            let files = Tagger::get_file_list(&path, subfolders);
            let rx = AudioFeatures::start_tagging(config, spotify, files);
            let start = timestamp!();
            for status in rx {
                debug!("{status:?}");
            }
            info!("Tagging finished, took: {} seconds.", (timestamp!() - start) / 1000);
        },
        // Spotify OAuth flow
        Actions::AuthorizeSpotify { client_id, client_secret, expose, prompt } => {
            let (auth_url, mut oauth) = Spotify::generate_auth_url(&client_id, &client_secret);
            println!("\nPlease go to the following URL and authorize 1T:\n{auth_url}");
            // should cache the token
            match prompt {
                true => {
                    println!("\nEnter the URL you were redirected to and press enter: ");
                    let mut url = String::new();
                    std::io::stdin().read_line(&mut url).expect("Couldn't read from stdin!");
                    let _spotify = Spotify::auth_token_code(&mut oauth, url.trim()).expect("Spotify authentication failed!");
                },
                false => {
                    let _spotify = Spotify::auth_server(&mut oauth, *expose).expect("Spotify authentication failed!");
                }
            }
            info!("Succesfully authorized Spotify!");
        }
    
    }
}


#[derive(Parser, Debug, Clone)]
#[clap(version)]
struct Cli {
    /// What should OneTagger do
    #[clap(subcommand)]
    action: Option<Actions>,
    
    /// Prints the default Autotagger config and exits
    #[clap(long)]
    autotagger_config: bool,

    /// Prints the default Audio Features config and exits
    #[clap(long)]
    audiofeatures_config: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Actions {
    /// Start Autotagger in CLI mode
    Autotagger {
        /// Path to music files (overrides config)
        #[clap(short, long)]
        path: String,

        /// Specify a path to config file
        #[clap(short, long)]
        config: Option<String>,

        /// Comma separated list of platforms to use. For custom platforms use the library filename
        #[clap(short = 'P', long)]
        platforms: Option<String>,

        /// Comma separated list of tags to use
        #[clap(short, long)]
        tags: Option<String>,

        /// Use ID3v2.4 instead of IDv2.3 for MP3/AIFF files
        #[clap(long)]
        id3v24: bool,

        /// Overwrite the existing tags in the track
        #[clap(long)]
        overwrite: bool,

        /// How many threads to use for the searching & matching process
        #[clap(long)]
        threads: Option<u16>,

        /// How strict should the matching be? Use: 0 - 100, Default: 80 (%).
        #[clap(long)]
        strictness: Option<u8>,

        /// Writes a cover.jpg into the folder
        #[clap(long)]
        album_art_file: bool,

        /// Merge new genres with existing ones
        #[clap(long)]
        merge_genres: bool,

        /// Write the key tag in CAMELOT format
        #[clap(long)]
        camelot: bool,

        /// Write title tag without version (ie. remix)
        #[clap(long)]
        short_title: bool,

        /// Match the song duration as well (WARNING: very strict)
        #[clap(long)]
        match_duration: bool,

        /// If duration matching is enabled, how big the difference in durations can be (in seconds)
        #[clap(long)]
        max_duration_difference: Option<u64>,

        /// Use platform specific ID tags to get exact matches
        #[clap(long)]
        match_by_id: bool,

        /// Try to indentify the track on Shazam if title & artist tags are missing
        #[clap(long)]
        enable_shazam: bool,

        /// Always try to indentify the track on Shazam
        #[clap(long)]
        force_shazam: bool,

        /// Skip tracks that have 1T_TAGGEDDATE tag
        #[clap(long)]
        skip_tagged: bool,

        /// Try to get title & artist from filename if the tags are missing
        #[clap(long)]
        parse_filename: bool,

        /// Template for parse_filename option. Example: `%track$. %artists% - %title%`
        #[clap(long)]
        filename_template: Option<String>,

        /// Don't include subfolders
        #[clap(long)]
        no_subfolders: bool,
    },
    /// Start Audio Features in CLI mode
    Audiofeatures {
        /// Path to music files (overrides config)
        #[clap(short, long)]
        path: String,

        /// Specify a path to config file
        #[clap(short, long)]
        config: String,

        /// Spotify Client ID
        #[clap(long)]
        client_id: String,

        /// Spotify Client Secret
        #[clap(long)]
        client_secret: String,

        /// Don't include subfolders
        #[clap(long)]
        no_subfolders: bool,
    },
    /// Authorize Spotify and cache the token
    AuthorizeSpotify {
        /// Spotify Client ID
        #[clap(long)]
        client_id: String,
        
        /// Spotify Client Secret
        #[clap(long)]
        client_secret: String,

        /// Run Spotify authentication callback server on `0.0.0.0`
        #[clap(long)]
        expose: bool,

        /// Don't start server, prompt for the redirected URL 
        #[clap(long)]
        prompt: bool
    }
}

/// For easily generating the tags string to config
macro_rules! contains_tag_config {
    ($target:expr, $source:expr, $t:tt) => {
        $target.$t = $source.contains(&stringify!($t))
    };
    ($target:expr, $source:expr, $($t:tt),+) => {
        $(contains_tag_config!($target, $source, $t);)+
    }
}

/// For easily generating CLI -> config
macro_rules! config_option {
    ($target:expr, $t:tt) => {
        if *$t {
            $target.$t = *$t;
        }
    };
    ($target:expr, $($t:tt),+) => {
        $(config_option!($target, $t);)+
    }
}

impl Actions {
    //. Create tagger config
    pub fn get_at_config(&self) -> Result<TaggerConfig, Box<dyn Error>> {
        match self {
            Actions::Autotagger { path, config, platforms, tags, id3v24, 
                overwrite, threads, strictness, album_art_file, merge_genres, camelot, 
                short_title, match_duration, max_duration_difference, match_by_id, enable_shazam, force_shazam, 
                skip_tagged, parse_filename, filename_template, no_subfolders } => {

                // Load config
                let mut config = if let Some(config_path) = config {
                    let config = serde_json::from_reader(&File::open(config_path)?)?;
                    config
                } else {
                    TaggerConfig::custom_default()
                };

                // Overrides
                config.path = Some(path.to_string());
                if let Some(platforms) = platforms {
                    config.platforms = platforms.split(",").map(String::from).collect();
                }
                // Tags
                if let Some(tags) = tags {
                    let tags: Vec<_> = tags.split(",").collect();
                    contains_tag_config!(config, tags, title, artist, album, key, bpm, genre, style, label, release_date, 
                        publish_date, album_art, other_tags, catalog_number, url, track_id, release_id, version, duration, 
                        album_artist, remixer, track_number, isrc, meta_tags);
                }
                // Boolean options
                config_option!(config, id3v24, overwrite, album_art_file, merge_genres, camelot, short_title, match_duration,
                    match_by_id, enable_shazam, force_shazam, skip_tagged, parse_filename);
                // Remaining options
                if let Some(threads) = threads {
                    config.threads = *threads;
                }
                if let Some(strictness) = strictness {
                    if *strictness > 100 {
                        warn!("Invalid stricness!");
                    } else {
                        config.strictness = *strictness as f64 / 100.0;
                    }
                }
                if let Some(mdd) = max_duration_difference {
                    config.max_duration_difference = *mdd;
                }
                if let Some(template) = filename_template {
                    config.filename_template = Some(template.to_string());
                }
                if *no_subfolders {
                    config.include_subfolders = false;
                }
                return Ok(config);
            },
            _ => unreachable!()
        }
    }
}

