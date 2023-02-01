use async_compression::tokio::bufread::{
    GzipDecoder as ReaderGzipDecoder, GzipEncoder as ReaderGzipEncoder,
};
use clap::{Parser, Subcommand};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::{metadata as async_metadata, File as AsyncFile};
use tokio::io::{
    AsyncReadExt, AsyncWriteExt, BufReader as TokioBufReader, BufWriter as TokioBufWriter,
    Error as TokioIOError, Result as TokioIOResult,
};
use tokio::sync::Semaphore;
use tokio::task::JoinError as TokioJoinError;

async fn is_file(path: &Path) -> bool {
    let metadata = async_metadata(path).await;
    if let Ok(metadata) = metadata {
        metadata.is_file()
    } else {
        false
    }
}

async fn gzip(path: &Path, keep_original: bool) -> TokioIOResult<()> {
    // Define the buffer for the compressed data, the reader, and the encoder
    let mut buffer = Vec::new();
    let reader = TokioBufReader::new(AsyncFile::open(path).await?);
    let mut encoder = ReaderGzipEncoder::new(reader);

    // Read the compressed data into the buffer
    encoder.read_to_end(&mut buffer).await?;

    // Define the output path and the writer
    let output_path = format!("{}.gz", path.to_string_lossy());
    let mut writer = TokioBufWriter::new(AsyncFile::create(&output_path).await?);

    // Write the compressed data to the output file and shutdown the writer
    writer.write_all(&buffer).await?;
    writer.shutdown().await?;

    // Delete the original file if keep_original is false (default behavior)
    if !keep_original {
        tokio::fs::remove_file(path).await?;
    }

    Ok(())
}

async fn unzip(path: &Path, keep_original: bool) -> TokioIOResult<()> {
    // Define the buffer for the decompressed data, the reader, and the decoder
    let mut buffer = Vec::new();
    let reader = TokioBufReader::new(AsyncFile::open(path).await?);
    let mut decoder = ReaderGzipDecoder::new(reader);

    // Read the decompressed data into the buffer
    decoder.read_to_end(&mut buffer).await?;

    // Define the output path and file, and the writer
    let output_path = path.with_extension("");
    let output_file = tokio::fs::File::create(&output_path).await?;
    let mut writer = TokioBufWriter::new(output_file);

    // Write the decompressed data to the output file and shutdown the writer
    writer.write_all(&buffer).await?;
    writer.shutdown().await?;

    // Delete the original file if keep_original is false (default behavior)
    if !keep_original {
        tokio::fs::remove_file(path).await?;
    }

    Ok(())
}

/// A simple utility for compressing and decompressing files using the Gzip algorithm in a multithreaded manner.
#[derive(Parser, Debug)]
#[command(name = "super-gunzip", version)]
struct SuperGunzip {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compresses all files matching the given pattern using the Gzip algorithm.
    /// Appends a .gz extension to the compressed files
    Gzip {
        /// The glob-like pattern to match files against
        #[arg()]
        pattern: String,

        /// Whether to keep the original files after compression. By default, the original files are deleted.
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        keep_original: bool,

        /// The maximum number of threads to split the compression across (default: 1)
        #[arg(short, long)]
        num_threads: Option<usize>,

        /// Whether to be verbose about the decompression process
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        verbose: bool,
    },

    /// Decompresses all files matching the given pattern using the Gzip algorithm.
    /// Removes the .gz extension from the decompressed files
    Unzip {
        /// The glob-like pattern to match files against
        #[arg()]
        pattern: String,

        /// Whether to keep the original gzipped files after decompression. By default, the original files are deleted.
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        keep_original: bool,

        /// The maximum number of threads to split the decompression across (default: 1)
        #[arg(short, long)]
        num_threads: Option<usize>,

        /// Whether to be verbose about the decompression process
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        verbose: bool,
    },
}

#[derive(Debug)]
enum SuperGzipError {
    IO(TokioIOError),
    Threading(TokioJoinError),
}

impl From<TokioIOError> for SuperGzipError {
    fn from(src: TokioIOError) -> Self {
        Self::IO(src)
    }
}

impl From<TokioJoinError> for SuperGzipError {
    fn from(src: TokioJoinError) -> Self {
        Self::Threading(src)
    }
}

async fn _wrapper(
    b_zip: bool,
    pattern: String,
    keep_original: bool,
    num_threads: Option<usize>,
    verbose: bool,
) -> Result<(), String> {
    let start = Instant::now();
    let mut errors: Vec<SuperGzipError> = vec![];
    let _max_threads = num_threads.unwrap_or(1);
    let semaphmore = Arc::new(Semaphore::new(_max_threads));
    let paths =
        glob::glob(&pattern).expect("Invalid glob pattern provided. Please check your input.");
    let mut handles = Vec::new();
    for path in paths.flatten() {
        let resource_lock = Arc::clone(&semaphmore);
        let handle = tokio::spawn(async move {
            // Silently return if the path is not a file
            if !is_file(&path).await {
                return Ok(());
            }

            // Silently return if the path extension doesn't fit the compression/decompression criteria
            if (b_zip && path.extension() == Some("gz".as_ref()))
                || (!b_zip && path.extension() != Some("gz".as_ref()))
            {
                if verbose {
                    println!("Skipping {}", path.to_string_lossy());
                }
                return Ok(());
            }

            let _permit = resource_lock.acquire_owned().await.expect("Failed to acquire permit from semaphore. This is a bug in the program. Please report it.");
            let result = if b_zip {
                if verbose {
                    println!("Compressing {}", path.to_string_lossy());
                }
                gzip(&path, keep_original).await
            } else {
                if verbose {
                    println!("Decompressing {}", path.to_string_lossy());
                }
                unzip(&path, keep_original).await
            };
            drop(_permit);
            result
        });
        handles.push(handle);
    }
    for handle in handles {
        let join_result = handle.await;
        match join_result {
            Ok(gzip_result) => {
                if let Err(gzip_error) = gzip_result {
                    errors.push(gzip_error.into());
                }
            }
            Err(join_error) => {
                errors.push(join_error.into());
            }
        }
    }
    if verbose {
        println!("Finished in {} seconds", start.elapsed().as_secs_f64());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        println!("Finished with {} errors.", errors.len());
        Err(errors
            .into_iter()
            .map(|e| format!("{:?}", e))
            .collect::<Vec<String>>()
            .join("\n"))
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args = SuperGunzip::parse();
    match args.commands {
        Commands::Gzip {
            pattern,
            keep_original,
            num_threads,
            verbose,
        } => {
            return _wrapper(true, pattern, keep_original, num_threads, verbose).await;
        }
        Commands::Unzip {
            pattern,
            keep_original,
            num_threads,
            verbose,
        } => {
            return _wrapper(false, pattern, keep_original, num_threads, verbose).await;
        }
    }
}
