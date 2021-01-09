import gzip
import argparse
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from shutil import copyfileobj


def base_file_gunzip(orig_path: Path, unzip: bool = False, retain_original: bool = False, verbose: bool = False):
    """
    Standard zip or unzip a given filepath to a file

    :param orig_path: the path to the file to be zipped or unzipped
    :param unzip: whether to unzip the file (default: False, zipping is performed)
    :param retain_original: whether to retain the original file or not
    :param verbose: whether to give print statements (default: False)
    :return: status of the operation (True = successful) and the original filepath
    """
    if verbose:
        print(f"{'Decompressing' if unzip else 'Compressing'} {str(orig_path)}")

    # Sanity Checks
    if not orig_path.exists():
        print(f"The path specified: {orig_path} does not exist")
        return False, str(orig_path)
    if orig_path.is_dir():
        print(f"The path specified: {orig_path} is a directory. This program only supports files")
        return False, str(orig_path)

    # Unzip action
    if unzip and orig_path.name.endswith(".gz"):
        new_path = Path(str(orig_path)[:-3])
        with gzip.open(orig_path, "rb") as f_in:
            with open(new_path, "wb") as f_out:
                copyfileobj(f_in, f_out)
    # Zip action
    elif not unzip and not orig_path.name.endswith(".gz"):
        new_path = Path(str(orig_path) + ".gz")
        with open(orig_path, "rb") as f_in:
            with gzip.open(new_path, "wb") as f_out:
                copyfileobj(f_in, f_out)

    # Otherwise return
    else:
        print(f"Check run options. Neither criteria for zipping nor unzipping found for {str(orig_path)}")
        return False, str(orig_path)

    # Delete the original if retaining it is not specified
    if not retain_original:
        orig_path.unlink(missing_ok=True)

    return True, str(orig_path)


def str2bool(a_string):
    """
    Helper function to interpret a string as a boolean

    :param a_string: the string to interpret
    :return: True or False
    """
    if isinstance(a_string, bool):
        return a_string
    if a_string.lower() in ('yes', 'true', 't', 'y', '1'):
        return True
    elif a_string.lower() in ('no', 'false', 'f', 'n', '0'):
        return False
    else:
        raise argparse.ArgumentTypeError('Boolean value expected.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description="A multithreaded implementation of Python's builtin Gzip")
    parser.add_argument("root_dirs", help="These are the directories to base the glob search off of", nargs="+")
    parser.add_argument("pattern", help="This is glob pattern to use when searching for files to zip/unzip", nargs=1)
    parser.add_argument("-d", "--decompress", nargs=1, default=False,
                        help="Whether to decompress [1 | True | T] or compress (default) "
                             "[0 | False | F] any located files")
    parser.add_argument("-k", "--keep_orig", nargs=1, default=False,
                        help="Whether to keep [1 | True | T] or discard (default) [0 | False | F] the original file")
    parser.add_argument("-r", "--recursive", nargs=1, default=True,
                        help="Whether to search recursively (default) [1 | True | T] or non-recursively "
                             "[0 | False | F] when globbing for target files from the indicated root directories")
    parser.add_argument("-n", "--n_threads", type=int, nargs=1, default=4,
                        help="This is the number of threads to call when performing the required operation")
    parser.add_argument("-p", "--pause", nargs=1, default=True,
                        help="Whether to pause and review commands before proceeding (default) [1 | True | T] "
                             "or skip straight to processing [0 | False | F]")
    parser.add_argument("-v", "--verbose", nargs=1, default=False,
                        help="Whether to give stdout feedback [1 | True | T] as files are being compressed or "
                             "decompressed or whether to remain silent (default) [0 | False | F]")

    # Prepare variables
    args = parser.parse_args()
    roots: list = [Path(path).resolve() for path in args.root_dirs if Path(path).resolve().is_dir()]
    search_pat: str = args.pattern[0]
    b_decompress: bool = args.decompress if isinstance(args.decompress, bool) else str2bool(args.decompress[0])
    b_keeporig: bool = args.keep_orig if isinstance(args.keep_orig, bool) else str2bool(args.keep_orig[0])
    b_pause: bool = args.pause if isinstance(args.pause, bool) else str2bool(args.pause[0])
    b_recursive: bool = args.recursive if isinstance(args.recursive, bool) else str2bool(args.recursive[0])
    b_verbose: bool = args.verbose if isinstance(args.verbose, bool) else str2bool(args.verbose[0])
    if isinstance(args.n_threads, int):
        nthreads: int = args.n_threads
    elif isinstance(args.n_threads, list):
        nthreads: int = int(args.n_threads[0])
    else:
        raise ValueError("Bad argument supplied to n_threads parameter. Please supply an integer value")

    # Sanity check
    if b_decompress and not search_pat.endswith(".gz"):
        raise ValueError("A decompression (gunzip) job was specified but the search pattern did not end in .gz")
    if not b_decompress and search_pat.endswith(".gz"):
        raise ValueError("A compression (gzip) job was specified but the search pattern included .gz files. Cannot "
                         "further gzip files that have already been gzipped.")

    # Prepare a printable representation of the variables if the user chooses to pause and keep them in a choice loop
    if b_pause:
        repr_dirs_to_search = "\n\t".join([str(path) for path in roots])
        print(f"Arguments are as follows:\n"
              f"Directories to search:\n\t{repr_dirs_to_search}\n"
              f"Glob Pattern: {search_pat}\n"
              f"Number of threads called: {nthreads}\n"
              f"Whether to decompress located files: {b_decompress}\n"
              f"Whether to keep the original files: {b_keeporig}\n"
              f"Whether to act verbously: {b_verbose}\n")
        choice = ""
        while choice not in ["y", "n"]:
            choice = input("[y|n] Proceed? ")
            if choice.lower() == "n":
                exit(0)
            elif choice.lower() == "y":
                break
            else:
                print("Sorry, that is not a valid choice")
                continue

    # Either if the choice loop is exited or skipped, proceed to the gzipping or unzipping
    with ThreadPoolExecutor(max_workers=nthreads) as executor:
        for root_path in roots:
            paths = root_path.rglob(pattern=search_pat) if b_recursive else root_path.glob(pattern=search_pat)
            for path in paths:
                if path.is_file():
                    executor.submit(base_file_gunzip, path, b_decompress, b_keeporig, b_verbose)
