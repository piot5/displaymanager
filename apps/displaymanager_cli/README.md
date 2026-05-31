Display Manager CLI

Usage examples:

- Scan connected displays:

```bash
flux-cli display --scan
```

- Enable a display by friendly name (uses `df_displmgr_info` synthesized registry):

```bash
flux-cli display --output "My Monitor" --mode 1920x1080 --pos 0x0 --rotate 0
```

- Turn off a display by device path:

```bash
flux-cli display --output "\\.\\DISPLAY1" --off
```

- Set position using comma or 'x' separator:

```bash
flux-cli display --output "My Monitor" --pos 100,200
flux-cli display --output "My Monitor" --pos 100x200
```

- Set rotation (accepted values: 0, 90, 180, 270):

```bash
flux-cli display --output "My Monitor" --rotate 90
```

- DDC examples (use the numeric index shown by `flux-cli ddc list`):

```bash
flux-cli ddc --id 0 list
flux-cli ddc --id 0 brightness --value 80
flux-cli ddc --id 0 input --source hdmi1
flux-cli ddc --id 0 input --source 0x11
```
