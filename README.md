# transmission-rss
A simple binary that reads a config file with a list of rss torrent items and adds them
to transmission.

```
$ transmission-rss
USAGE:
    transmission-rss --config <CONFIG>

OPTIONS:
    -c, --config <CONFIG>    Path to the config file
    -h, --help               Print help information
    -V, --version            Print version information
                                                        
```

### Getting started
If you have cargo installed it's possible to install the binary by running:

```
$ cargo install transmission-rss
$ transmission-rss -c config.toml
```

### Config file

Example of `config.toml`:

```toml
base_download_dir = "/downloads/"

[persistence]
path = "/path/to/db/folder"

[transmission]
url = "http://myserver/transmission/rpc"
username = "myusername"
password = "mypassword"

[[rss_list]]
title = "My List"
url = "https://someweb.site/rss.xml"
filters = ["1080p"]
download_dir = "/downloads/my_folder"

[[rss_feeds]]
title = "My Feed"
url = "https://someweb.site/rss.xml"

[[rss_feeds.rules]]
filter = "1080p"
download_dir = "1080p"

[[rss_feeds.rules]]
filter = "4k"
download_dir = "4k"
```

The password can optionally be loaded from a separate file by specifying `password_file` instead.
