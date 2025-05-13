# Settings
Muzak can be configured with a `settings.json` file located in the following places:

| Platform | Location                                                          |
|----------|-------------------------------------------------------------------|
| Linux    | `~/.local/share/muzak/settings.json`                              |
| macOS    | `~/Library/Application Support/me.william341.muzak/settings.json` |
| Windows  | `%appdata%\william341\muzak\data\settings.json`                   |

## Example

```json
{
  "scanning": {
    "paths": ["/home/me/Music", "/home/me/other"]
  }
}
```

## Last.FM
The current Last.FM session is stored in the following places:

| Platform | Location                                                        |
|----------|-----------------------------------------------------------------|
| Linux    | `~/.local/share/muzak/lastfm.json`                              |
| macOS    | `~/Library/Application Support/me.william341.muzak/lastfm.json` |
| Windows  | `%appdata%\william341\muzak\data\lastfm.json`                   |

Deleting this file will disconnect your Last.FM account. This file should not
be modified manually - it will be generated when you connect your Last.FM
account.
