# i3toolwait

Launch a program and move it to the correct workspace.

## Usage

- **simple:** `i3toolwait simple ...`
- **config:** `i3toolwait config ...`

### Simple

Run only one program.

### Config

Run multiple programs by specifying a yaml configuration file:

```yaml
---
signal: signal number or name, optional. Should program entries which have signal: true wait for this signal before continuing to the next one.
timeout: timeout in milliseconds
programs:
- match: a filter with which to match the window
  workspace: string or null, the workspace to move windows to
  cmd: string or list, the command to execute
  signal: boolean, should we wait before continuing with the next entry
  timeout: timeout in milliseconds, used only if signal: true - how long to wait for the signal
```

The programs will be started asynchronously, except when `signal = true` which means that, before continuing
to the next program we wait for a signal. I would start all programs, which do not wait for a signal first
and then only the ones depending on the signal to reduce the startup delay.

## Installing

Use the makefile: `INSTALL_BASE=/usr/local/ make install` or install all dependencies
`python3 -mpip install --upgrade -r requirements.txt` and copy the script to your
path: `cp i3toolwait /usr/local/bin/i3toolwait`.

## Filtering

The program allows to match the window or container based on the returned IPC data.
Some programs might open multiple windows (looking at you, Discord).

In order to move the correct window to the desired workspace a filter can be defined.

The syntax for the filter is lisp-like. To view all spawned containers run the program
with `--debug --filter=False` which will not match any windows and print their properties.

It is then possible to construct a filter for any program.

Available Operators:

- and: `&`
- or: `|`
- eq: `=`
- neq: `!=`
- gt: `>`
- lt: `<`

The filter usually operates on the dictionary, and thus the *first* argument to every normal filter
is the dictionary element, in `.` notation, as might be customary in `jq`.

For example: `(> ".container.geometry.width" 300)` would match the first window where the width is greater than 300.

Multiple filters are combined via nesting: `(& (> ".container.geometry.width" 300) (= ".container.window_properties.class" "discord"))`.

## Starting tray programs in a specific order

To start tray programs in a specific order it is possible to specify the `signal` parameter.
Starting of programs will be halted until the program has received the corresponding signal.

This could be combined with waybar to enforce an ordering of tray applications:

`~/.config/waybar/config`
```json
"tray": {
    "on-update": "pkill --full --signal SIGUSR1 i3toolwait",
    "reverse-direction": true,
}
```

`config-file`
```yaml
signal: SIGUSR1
timeout: 2000
programs:
- cmd: 'nm-applet --indicator'
  match: '(False)'
  timeout: 1000
  signal: true
- cmd: 'blueman-applet'
  match: '(False)'
  timeout: 1000
  signal: true
- ...
```

This setup would order the icons in waybar from left-to-right like in the config file.

## Troubleshooting

### My windows do not get rearranged

It is very likely that the timeout is too short and the program exits before the window spawns.
Alternatively your filter might just be wrong. To debug execute the script with the `--debug`
flag to see if the window is recognized.
