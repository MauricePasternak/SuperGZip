# SuperGzip

## Table of Contents

- [About](#about)
- [Getting Started](#getting_started)
- [Usage](#usage)
- [Examples](#examples)


## About <a name = "about"></a>

This is a command line tool for batch gunzip behavior using multithreading spread across multiple files matched by a user-provided glob pattern.


## Getting Started <a name = "getting_started"></a>

Head on over to [the Releases section](https://github.com/MauricePasternak/SuperGZip/releases) of the repository and download the appropriate version for your operating system. Unzip and use in a command line terminal at your own discretion.


## Usage <a name = "usage"></a>

General syntax is as follows:

```bash
super-gunzip <gzip | unzip> <glob pattern> [options]
```

Where current options are:

- `-h` or `--help`: If this tag is present, the program will print the help message and exit.
- `-k` or `--keep_original`: If this tag is present, the program will not attempt to remove the original file.
- `-n <number>` or `--num_threads <number>`: The number of threads to split the workload across. It is the responsibility of the user to ensure that the number provided is reasonable. **Defaults to 1.**
- `-v` or `--verbose`: If this tag is present, the program will print the name of each file as it is processed.

:information_source: **Note:** On Unix, the use of the wildcard `*` character will require **the use of quotes** to prevent the shell from expanding the pattern before it is passed to the program. On Windows, this is not necessary. See the examples below.

## Examples <a name = "examples"></a>

```bash
# Get more information about the tool
super-gunzip --help
super-gunzip gzip --help
super-gunzip unzip --help

# Unzip or gzip all files in an indicated directory
super-gunzip gzip "some/filepath/glob/pattern*"
super-gunzip unzip "some/filepath/glob/pattern*.gz"

# Utilize multithreading
super-gunzip gzip "some/filepath/glob/pattern*" --num-threads 12
super-gunzip unzip "some/filepath/glob/pattern*.gz" --num-threads 12
```
