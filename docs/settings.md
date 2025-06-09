# Settings
Hummingbird can be configured with a `settings.json` file located in the following places:

| Platform | Location                                                              |
|----------|-----------------------------------------------------------------------|
| Linux    | `~/.local/share/hummingbird/settings.json`                            |
| macOS    | `~/Library/Application Support/org.mailliw.hummingbird/settings.json` |
| Windows  | `%appdata%\mailliw\hummingbird\data\settings.json`                    |

> [!NOTE]
> The default data directory was chanaged when Muzak was renamed to Hummingbird.
>
> If you first opened the application before the name change, your configuration files may
> be in the previous location.
>
> <details>
> <summary>Legacy (pre-Hummingbird) folder location</summary>
> <br>
>
> | Platform | Location                                                          |
> |----------|-------------------------------------------------------------------|
> | Linux    | `~/.local/share/muzak/settings.json`                              |
> | macOS    | `~/Library/Application Support/me.william341.muzak/settings.json` |
> | Windows  | `%appdata%\william341\muzak\data\settings.json`                   |
>
> This can be applied to all paths - they have all been changed in the same manner.
> </details>

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

| Platform | Location                                                            |
|----------|---------------------------------------------------------------------|
| Linux    | `~/.local/share/hummingbird/lastfm.json`                            |
| macOS    | `~/Library/Application Support/org.mailliw.hummingbird/lastfm.json` |
| Windows  | `%appdata%\mailliw\hummingbird\data\lastfm.json`                    |

Deleting this file will disconnect your Last.FM account. This file should not
be modified manually - it will be generated when you connect your Last.FM
account.
