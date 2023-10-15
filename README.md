# i3toolwait

Launch a program and move it to the correct workspace.

## Usage

`i3toolwait -c FILE`

Optionally start multiple programs and wait for their windows to appear.
Once these windows appeared a custom i3 command can be specified.

## Example

```yaml
---
timeout: 10000
init: |
  (begin
    (define i3_path ".container.window_properties.class")
    (define sway_path ".container.app_id")
    (defun idmatch (name) (== (if (has-key sway_path) (load sway_path) (load i3_path)) name))
    (defun match (name) (and (== (load ".change") "new") (idmatch name)))
    (defun match-load (name) (if (match name) (load ".container.id") F))
  )
cmd: 'workspace 1'
programs:
- run: 'exec gtk-launch librewolf'
  cmd: 'for_window [con_id="{result}"] focus; move container to workspace 1'
  match: '(match-load "LibreWolf")'
- run: 'exec gtk-launch nheko || gtk-launch io.element.Element'
  cmd: 'for_window [con_id="{result}"] focus; move container to workspace 2'
  match: '(if (or (match "Electron") (match "nheko")) (load ".container.id") F)'
- run: 'exec gtk-launch thunderbird'
  cmd: 'for_window [con_id="{result}"] focus; move container to workspace 3'
  match: '(match-load "thunderbird")'
- run: 'exec nm-applet --indicator'
- run: 'exec blueman-applet'
- run: 'exec gtk-launch org.kde.kdeconnect.nonplasma'
- run: 'exec gtk-launch syncthing-gtk'
```

## Configuration

The configuration file is in YAML format.


### Configuration

#### timeout: int

_Optional_ _Default_ `3000`

Total program timeout in ms.

#### init: String

_Optional_ _Default_ `""`

Initialization program; Used to initialize the environment, useful
to define custom functions which should be available everywhere.

#### cmd: String

_Optional_ _Default_ `""`

A final i3 command to be executed before exiting.

#### programs: List[Union[[Program](#program), [Signal](#signal)]]

_Optional_ _Default_ `[]`

A list of programs to execute.

### Program

Launch all programs using [`run`](#run-string) and execute
[`cmd`](#cmd-string-1) once [`match`](#match-string) matches
a window.

#### match: String

_Required_

A lisp program which analyzes the i3 window event and returns a value.
If the return value is `false` the window does not match and no
further processing occurs. Otherwise the i3 command
[`cmd`](#cmd-string-1).
will be executed.

#### cmd: String

_Required_

A i3 command. Can contain a format `{result}` which gets replaced
by the output of the match command.

**Example:**

`for_window [con_id="{result}"] focus; move container to window 1`

#### run: String

_Optional_ _Default_ `null`

A i3 command which is run at program startup, can be used to launch
programs.

**Example:**

`exec gtk-launch firefox`

### Signal

Programs are launched in order and only advance after
[`timeout`](#timeout-int-1) or after receiving signal
`SIGUSR1`.

#### run: String

_Optional_ _Default_ `null`

A i3 command.

#### timeout: int

_Optional_ _Default_ `500`

How long to wait for the signal in ms.
