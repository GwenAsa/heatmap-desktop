# Install and upgrade

## Zorin OS 18 / Ubuntu 24.04

### `.deb` (recommended)
```bash
cd ~/Downloads
sudo apt install ./Heatmap_<version>_amd64.deb
```
`apt` also installs the required libraries (`libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`). A harmless note such as *"Download is performed unsandboxed as root"* can appear when installing from your home folder.

### AppImage
```bash
sudo apt install libfuse2t64        # once
chmod +x Heatmap_<version>_amd64.AppImage
./Heatmap_<version>_amd64.AppImage
```

## Upgrade
Releases from 0.2.0 on carry increasing version numbers, so a plain `sudo apt install ./Heatmap_<new>_amd64.deb` upgrades in place. If you are moving from a build that reused the same version number, remove first:
```bash
sudo apt remove heatmap && sudo apt install ./Heatmap_<new>_amd64.deb
```
Quit the app (tray icon → Quit, or `pkill -x heatmap-app`) before upgrading. On first start of a newer version the database is upgraded automatically; **take a backup first** (*Backup DB*) because older versions cannot open an upgraded database.

## Your data
`~/.local/share/app.heatmap.desktop/heatmap.db` — not touched by install, upgrade or removal. Automatic safety copies before JSON imports are in `backups/` next to it.

## Troubleshooting
| Problem | Try |
|---|---|
| Blank or white window | `WEBKIT_DISABLE_COMPOSITING_MODE=1 heatmap-app` |
| No tray icon | Your shell needs AppIndicator support; use `heatmap-app --widget ring` or `--quick` instead |
| `Ctrl+Alt+H` does nothing | Wayland blocks global shortcuts: bind `heatmap-app --quick` in *Settings → Keyboard* |
| Widget stays above/below windows unexpectedly | Cycle the 📌 button; behaviour depends on the window manager |

## Command line
`heatmap-app [--widget ring|month|strip|sheet] [--quick] [--show] [--background]`
