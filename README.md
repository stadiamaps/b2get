# b2get

`b2get` is a stupidly simple CLI for downloading files from Backblaze B2.
It doesn't do anything else; you'll find none of the rest of the API is covered.
It's _just_ a file downloader.
It just downloads one file and exits.

## Why?

Mostly because we have had random issues with the official Python solution over the years,
and wanted something that was much simpler to debug
which could be distributed as a standalone binary.

The primary intended use case is in automation,
such as Ansible playbooks, in docker images, etc. for doing data downloads.

## Quickstart

1. Install via `cargo install b2get`. (Or download the release binary from GitHub; some OS/arch configs supported, but limited by free runners.)
2. Set `B2_APPLICATION_KEY_ID` and `B2_APPLICATION_KEY` environment variables with your B2 credentials.
3. Invoke via your preferred method giving the bucket name, file name in B2, and path on disk you want to save the file to.

```shell
b2get com-your-bucket remote-filename.tar path/to/local-path.tar
```

Run with `-h` or `--help` for  all options.

## Notes and Limitations

We developed this for our own internal use,
and that may result in a few quirks,
which we highlight below.

### We assume this is for use on a server or similar

We assume that you have a solid network connection and don't need to open up 8 parallel threads to b2.
We also rely on the underlying HTTP stack / `reqwest` for all error handling.

### Written contents on disk are NOT verified separately

Other clients often verify the data on disk directly,
which results in a lot of extra I/O and CPU.
This can take many minutes, during which a CPU core is saturated,
when downloading a large file.

Our approach is to compute the SHA1 hash as the file is downloading.
This eliminates the huge pause at the end and is extremely low overhead.
In particular:

* We assume that the RAM of your server can detect/correct errors
* We assume that you are using a filesystem like ZFS with atomic writes (even if asynchronously) and that it isn't corrupting bits
* So, when we finally sync the buffer at the end of the loop, we believe the file has been verified fully without a second pass

### It's REALLY simple

We currently don't support anything other than a single file download, and a very few (mostly inconsequential) options.
But we think this is one of the most attractive features.
It's extremely easy to understand, audit, and maintain compared to other tools.
PRs welcome if you'd like to extend it within the ethos of being a simple downloader.
