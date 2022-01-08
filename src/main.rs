#![allow(unused_imports, unused_mut, unused_variables, dead_code, unreachable_code)]
extern crate linked_hash_set;
use octocrab::Page;
use tokio::task::JoinHandle;
use linked_hash_set::LinkedHashSet;
use std::error::Error;
use structopt::StructOpt;
use octocrab::Octocrab;
use octocrab::{models, params, repos::RepoHandler};
use std::{fs, path::PathBuf};
use log::{info, warn, error};
use log::LevelFilter;
use git2::Repository;
use futures::prelude::*;
use futures::future::{join_all, ok, err};
use std::{thread, time};
use std::sync::Arc;
use dotenv;

/// Search for and download repositories that match your query
#[derive(StructOpt, Debug)]
struct Options {
	/// The language to search for
	#[structopt(long)]
    language: String,

    /// The path to the directory to download repositories
    #[structopt(long, parse(from_os_str), default_value = "output")]
    output: PathBuf,

    /// Licenses to look for
    #[structopt(long)]
    license: Vec<String>,

    /// Don't download repositories, just list their urls
    #[structopt(long)]
    no_download: bool,

    /// How many repos to download/list? Defaulted to 10
    #[structopt(long, default_value = "10")]
    limit: usize,

    /// How many repos to download/list? Defaulted to 1
    #[structopt(long, default_value = "1")]
    threads: usize,

    /// Show verbose output
    #[structopt(long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	dotenv::dotenv().ok();
	let mut options = Options::from_args();

	if options.verbose {
		simple_logging::log_to_stderr(LevelFilter::Info);
	} else {
		simple_logging::log_to_stderr(LevelFilter::Error);
	}

	info!("{:?}", &options);

	if !options.no_download {
		info!("Creating {}", &options.output.to_string_lossy());
		fs::create_dir_all(&options.output)?;
	}

	info!("Fetching up to {} urls...", &options.limit);
	let urls = get_repo_urls(&options).await?;

	for url in &urls {
		println!("{}", url);
	}

	if !options.no_download {
		// NOTE: I'm not 100% that this is correct or the most optimal way of parallelizing this
		let handles: Vec<JoinHandle<_>> = urls
			.iter()
			.map(|url| {
				let url = url.clone();
				let output = options.output.clone();
				tokio::task::spawn_blocking(move || download_url(&url, &output))
			})
			.collect();
		join_all(handles).await;
	}

	info!("Finished :)");
	Ok(())
}

fn download_url(url: &String, output: &PathBuf) {
	info!("Starting to download {}", &url);
	let target = get_target(&url, output);
	match clone_url(url, &target) {
		Ok(_) => {}
		Err(e) => {
			error!("{}", e);
		}
	};
	info!("Finished downloading {}", &url);
}

fn get_target(url: &String, base_dir: &PathBuf) -> PathBuf {
	// NOTE: this assumes the url is valid and ends in .git
	let pieces = url.strip_suffix(".git").unwrap().split("/").collect::<Vec<&str>>();
	let target = base_dir.join(format!("{}_{}", pieces[pieces.len()-2], pieces[pieces.len()-1]));
	return target;
}

fn clone_url(url: &String, target: &PathBuf) -> Result<(), Box<dyn Error>> {
	if target.exists() {
		info!("{} already exists. Exiting early", &target.to_string_lossy());
		return Ok(());
	}

	info!("Creating {}", &target.to_string_lossy());
	fs::create_dir_all(&target)?;

	return match Repository::clone(url, target) {
		Ok(repo) => Ok(()),
		Err(e) => Err(Box::new(e))
	};
}

async fn get_repo_urls(options: &Options) -> Result<LinkedHashSet<String>, Box<dyn Error>> {
	let mut query = format!("language:{}", options.language);
	for license in &options.license {
		query.push_str(&format!(" license:{}", license));
	}
	info!("Using query: \"{}\"", &query);

	let octocrab = if let Ok(token) = std::env::var("GITHUB_TOKEN") {
		info!("Found GITHUB_TOKEN variable. Using it...");
		Arc::new(Octocrab::builder().personal_token(token.to_string()).build()?)
	} else {
		octocrab::instance()
	};

	let mut page = octocrab
		.search()
		.repositories(&query)
		.sort("stars")
		.order("desc")
		.send()
		.await?;

	loop {
		let mut urls = LinkedHashSet::new();
		if pull_items(&page, &mut urls, &options) {
			return Ok(urls);
		}

		// try to fetch the next page
		match octocrab.get_page::<models::Repository>(&page.next).await {
			Ok(Some(next_page)) => {
				page = next_page;
			}
			Ok(None) => {
				info!("Ran out of pages before we found enough matching urls");
				return Ok(urls);
			}
			Err(e) => {
				error!("Encountered error before we found enough matching urls: {}", e);
				return Ok(urls);
			}
		}
	};
}

// returns true if we're finished pulling items
fn pull_items(page: &Page<models::Repository>, urls: &mut LinkedHashSet<String>, options: &Options) -> bool {
	for item in &page.items {
		if let Some(url) = &item.clone_url {
			let url = url.to_string();
			if urls.contains(&url) {
				info!("Skipping {} because we've already seen it", &url);
				continue;
			}

			if !options.no_download && get_target(&url, &options.output).exists() {
				info!("Skipping {} because we already have it", &url);
				continue;
			}

			urls.insert(url);
			if urls.len() >= options.limit {
				return true;
			}
		}
	}
	return false;
}