# TUI Command Palette

The Soroban Debugger features a Command Palette in its interactive interfaces (TUI and REPL) to improve the discoverability of advanced features without requiring you to memorize keybindings or read extensive documentation.

## Opening the Command Palette

- **In the TUI (`soroban-debug tui`)**: Press `Ctrl+P` or `:` to open the command palette.
- **In the REPL (`soroban-debug repl`)**: Type `palette` and press Enter.

## Using the Command Palette

Once open, you can type to fuzzily search for actions. The palette will instantly filter the available commands based on your input.

### Example Actions

The palette provides access to advanced operations that might not have a dedicated UI button or simple keybinding, such as:

- **Export Trace**: Save the current execution trace to a JSON file.
- **Add Breakpoint**: Interactively set a breakpoint by selecting a function from a list.
- **Open Diagnostics**: View source map and DWARF diagnostic health reports.
- **Export Storage**: Save the current contract storage state to a snapshot file.
- **Change View**: Switch between specialized TUI views (e.g., Call Stack, Storage, Events).

## Keyboard Navigation

- Use the `Up` and `Down` arrow keys to navigate the filtered list of commands.
- Press `Enter` to execute the highlighted command.
- Press `Esc` to close the command palette without taking any action.

## Extensibility

As new features are added to the Soroban Debugger, they will be registered in the command palette. Plugin authors can also register their custom commands to appear in the palette, making them easily accessible to users.