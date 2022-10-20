# i3toolwait

Launch a program and move it to the correct workspace.

## Usage

- **simple:** `i3toolwait simple ...`
- **config:** `i3toolwait config ...`

### Simple

Run only one program.

### Config

Run multiple programs by specifying a yaml list of the form:

```yaml
---
- filter: <your filter>
  workspace: <target workspace>
  program: <program to execute>
```

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

