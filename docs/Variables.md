---
title: "Variables"
source: "https://github.com/xmichelo/Beeftext/wiki/Variables"
author:
  - "[[GitHub]]"
published:
created: 2026-04-01
description: "A text snippet tool for Windows. Contribute to xmichelo/Beeftext development by creating an account on GitHub."
tags:
  - "clippings"
---
## Definition

Variables are specially formatted text elements that can be placed in the substitution text of a combo. Every time Beeftext perform a combo substitution, the content of each variable is evaluated and the variable is replaced by the result of this evaluation.

## Example

Using variables, the substitution text `#{dateTime:yyyy-MM-dd}, #{combo:xxc}, Switzerland` can expand to `2018-01-30, Lausanne, Switzerland`, provided you have already defined a comb `xxc` that expands to `Lausanne`.

## Format

A variable is a piece of text enclosed between `#{` and `}`. In its simplest form, the text for a variable is a series of letters, the **variable name**, such as `#{dateTime}` and `#{time}`.

A variable can also have a parameter. In that case, the variable name is followed by a `:`, then by a character string, the **parameter**. The content of the parameter string depends on the variable name. For instance, in the `#{combo:xxcn}` the parameter `xxcn` must be the shortcut of another combo.

## Rules

- Variables are case sensitive. `#{dateTime}` is a valid variable, but `#{datetime}` and `#{DateTime}` are not.
- Extra white spaces are not allowed. `#{ dateTime }` is not a valid variable.
- If a variable is not recognized, it will not be replaced or removed.
- To use the `}` and `\` characters in a variable, you must 'escape' them by using `\{` and `\\`, respectively. Any other use of the `\` character will lead to undetermined results.

## List of available variables

## #{clipboard}

The `#{clipboard}` variable is replaced by the text currently stored in the system clipboard. If the clipboard contains rich text, the formatting is lost. If the clipboard is empty or contains anything other than text (an image or another form of binary data), the variable is simply ignored.

## #{date}

The `#{date}` variable is replaced by the current date. The format and language used depends on the system's region and language settings.

## #{time}

The `#{time}` variable is replaced by the current time. The format and language used depends on the system's region and language settings.

## #{dateTime}

The `#{dateTime}` variable is replaced by the current date and time. The format and language used depends on the system's region and language settings.

## #{dateTime:}

The `#{dateTime:<format>}` variable is replaced by the current date and/or time formatted according the content of the parameter *<format>*. For translatable portions (for instance the day of the week), the language used is the system language.

**Example**: `#{dateTime:yyyy-MM-dd, HH:mm:ss.zzz}` will evaluate to `2018-01-29 20:35:12.120`.

**Format specification**: Beeftext uses the [Qt framework](https://www.qt.io/). As a consequence the date & time format parameter uses the conventions of the Qt date and time functions. Please refer to the following pages in the Qt documentation for the complete list of available formatting options:

- [Time](https://doc.qt.io/qt-6/qtime.html#toString)
- [Date](https://doc.qt.io/qt-6/qdate.html#toString-3)

Starting with Beeftext 9.0, the format string can also include the week number using `ww` (week number with leading zero) and `w` (week number without leading zero). For instance, during the sixth week of the year:

- `The week number is #{dateTime:w}.` will be replaced by `The week number is 6.`
- `The week number is #{dateTime:ww}.` will be replaced by `The week number is 06.`

## #{dateTime::}

The third variant of the `{dateTime}` variables let you perform date and time calculation by adding or subtracting time to the current date/time. The format is `#{dateTime:<dateTimeShift>:<format>}`, where `<dateTimeShift>` is a sequence of addition and susbstraction such as `+1d-1w+3h`, which adds 1 day, remove 1 week and adds 3 hours to the current date. The available time specifiers are:

- y: year
- M: month
- w: week
- d: day
- h: hour
- m: minute
- s: second
- z: milliseconds

Example: `#{dateTime:+1d-1y+2h:yyyy-MM-dd hh:mm:ss}`: will be replaced by the current date 1 day, minus 1 year and plus 2 hours.

**Notes**

- The dateFormat parameter can be omitted, but the final colon is mandatory, e.g: `#{dateTime:+1d:}` will be replaced by the current date plus on days, formatted using the current system settings.
- The sign specifier, even when positive, is mandatory `#{dateTime:+1d:}` is valid, `#{dateTime:1d:}` is not.

## #{combo:}

The `#{combo:<keyword>}` variable is replaced by the snippet of the combo whose keyword is `<keyword>`. For example, if you have two combos:

- keyword: `xxfn` - snippet `John Fitzgerald`
- keyword: `xxln` - snippet `Kennedy`

Then you can create a third combo with the snippet `#{combo:xxfn} #{combo:xxln}` that will evaluate to `John Fitzgerald Kennedy`.

During the evaluation of a `#{combo:}` variable, variables within the sub-combos are also evaluated provided they do not create an endless loop. If an endless loop is detected, the #variable creating the loop is not evaluated.

## #{lower:} and #{upper:}

`#{lower:}` and `#{upper}` are equivalent to `#{combo:}`, but the result is converted to lower-case or upper-case, respectively. If we keep the example used above, `#{lower:xxfn} #{upper:xxln}` will be replaced by `john fitzgerald KENNEDY`.

## #{cursor}

The `#{cursor}` variable is a special variable. It is not evaluated. It is removed, but it determines the the position of the text cursor at the end of the substitution. If multiples occurrences of the `#{cursor}` are found, only the on first is used to determine the final position of the text cursor.

## #{input:}

The `#{input:<description>}` variable allow the user to interactively type text that will be inserted into a combo, using a standard message box. The text after the colon will be displayed in the input dialog. for instance, A combo containing `#{input:First name}` in the snippet will show the following message box:

![](https://github.com/xmichelo/Beeftext/wiki/assets/images/InputDialog.png)

## #{envVar:}

The `#{envVar:<variableName>}` variable is replaced by the content of the environment variable located after the colon, e.g. \`#{envVar:USERNAME} will be replaced by the login of the current Windows user.

## #{powershell:}

The `#{powershell:<pathToScript>}` variable executes the PowerShell script located at `<pathToScript>`, and inserts its output in the snippet. For instance if a user ComputerUser has the following script in a file located at `C:\Temp\script.ps1`:

```
Write-Host -NoNewLine ([System.Environment]::UserName)
```

the snippet `Hello #{powershell:C:\Temp\script.ps1}!` will evaluate to `Hello ComputerUser!`

Powershell scripts let the user perform advanced tasks. As a second example, here is a script that will retrieve the version number of the latest available version of Beeftext:

```
try 
{
   $response = Invoke-WebRequest -UseBasicParsing https://beeftext.org/latestVersionInfo.json | ConvertFrom-Json
   $result = "{0}.{1}" -f $response.'versionMajor', $response.'versionMinor'
} 
catch
{
   $result = "unknown" 
}
Write-Host -NoNewLine $result
```

By default, the execution of the script will be terminated by Beeftext if it does finish by itself within 10 seconds. Starting with Beeftext v11.0, this timeout mechanism can be modified or disabled. using the extended syntax `#{powershell:<pathToScript>:<timeoutMs>`

The `timeoutMs` value should be a positive integer, with no sign.

- if set to `0`, Beeftext will wait indefinitely for the script to terminate.
- if set to an non-zero position integer, Beeftext will terminate the script within `timeoutMs` milliseconds.

As an example, let's assume the following script (contributed by [th-schall](https://github.com/th-schall)) is stored in the file `C:\Temp\filename.ps1`.

```
Add-Type -AssemblyName System.Windows.Forms
$outstring = "No selection"
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.initialDirectory = "MyComputer"
$dialog.filter = "All Files (*.*)|*.*"
$response = $dialog.ShowDialog((New-Object System.Windows.Forms.Form -Property @{TopMost = $true }))
if ($dialog.Filename) { $outstring = Split-Path $dialog.Filename -leaf }
Write-Host -NoNewLine $outstring
```

This script display a file picking dialog and return the name of the selected file (without path).

- The snippet `The name of the file is "#{powershell:C:\Temp\filename.ps1}".` will display a dialog that will disappear automatically after 10 seconds.
- The snippet `The name of the file is "#{powershell:C:\Temp\filename.ps1:5000}".` will display a dialog that will disappear automatically after 5 seconds.
- The snippet `The name of the file is "#{powershell:C:\Temp\filename.ps1:0}".` will display a dialog that will only disappear when the user validates or cancels it.

## #{key:}

The key variable has been introduced in Beeftext v11.0. It allows to emulate the pressing of a non printable character key. for instance, the following snippet will generate a 1 and a 2 separated by a tab:

`1#{key:tab}2`

Optionally, you can specify the number of time the key should be repeated. The following snippet will emulate 10 times the Up arrow key:

`#{key:up:10}`

The list of available keys is:

- space
- tab
- enter
- insert
- delete
- home
- end
- pageUp
- pageDown
- up
- down
- left
- right
- escape
- printScreen
- pause
- numLock
- volumeMute
- volumeUp
- volumeDown
- mediaNextTrack
- mediaPreviousTrack
- mediaStop
- mediaPlayPause
- mediaSelect
- windows
- control
- alt
- shift
- f1... f24

**Note**: Modifier-key shortcuts (e.g., `Ctrl+A`, `Ctrl+Shift+J`) are not supported. Plain function keys (`F1`–`F24`) and media keys listed above are fully supported. Only the special `#{shortcut:}` variable can invoke keyboard shortcuts.

## #{shortcut:}

The shortcut variable was introduced in Beeftext v12.0. You can use it to invoke a shortcut from a snippet.

For instance the snippet `Before#{shortcut:Ctrl+Shift+J}After` paste `Before`,n invoke the Ctrl+Shift+J shortcut, then paste ` After`.

Be careful when using shortcuts, as they may interfere with the behavior of Beeftext (notably when manipulating the clipboard), and they are generally application dependent.

*Tip*: the combo editor's context menu for inserting a shortcut variable will display a window that will record your shortcut. It's helpful to avoid mistakes when typing the shortcut's description.

## #{delay:}

The `#{delay:}` variable has been introduced in Beeftext v11.0. It allows to pause the expansion of the snippet for a specified amount of milliseconds. The following snippet will paste `abc`, pause for 500 milliseconds, then paste `def`.

`abc#{delay:500}def`

## Context menu

All available variables can be easily inserted using the *Insert Variable* sub-menu in the snippet editor' s context menu.

![The 'Insert variable' context menu](https://github.com/xmichelo/Beeftext/wiki/assets/images/VariablesMenu.png)