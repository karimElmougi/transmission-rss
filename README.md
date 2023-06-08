# transmission-rss
A simple binary that reads a config file with a list of rss torrent items and adds them
to transmission.

### Getting started
If you have cargo installed it's possible to install the binary by running:

```
$ cargo install --git https://github.com/karimElmougi/transmission-rss
$ transmission-rss
```

### Config file

The config is located at `$HOME/.config/transmission-rss/config.toml`.

Example of `config.toml`:

```toml
base_download_dir = "/downloads/"

[transmission]
url = "http://myserver/transmission/rpc"
username = "myusername"
password = "mypassword"

[[rss_feeds]]
name = "Some RSS Feed"
url = "https://someweb.site/rss.rss.xml"

[[rss_feeds.rules]]
filter = "Some Show"
download_dir = "some_show/"
labels = ["TV", "Season 02"]
```

The password can optionally be loaded from a separate file by specifying `password_file` instead.
