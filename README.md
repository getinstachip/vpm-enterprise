# Verilog Package Manager (VPM)

VPM is a powerful package manager for Verilog projects, currently being piloted at Stanford and UC Berkeley. It's designed to streamline the management, reuse, and communication of IP cores and dependencies in hardware design workflows, significantly accelerating your design process.

## Features

- **Module Management**: Easily include, update, and remove modules in your project.
- **Documentation Generation**: Automatically create comprehensive documentation for your Verilog modules.
- **Dependency Handling**: Manage project dependencies with ease.
- **Simulation Support**: Simulate your Verilog files directly through VPM.
- **Tool Integration**: Seamlessly install and set up open-source tools for your project.

## Installation

VPM is designed for easy installation with no additional dependencies. 

### Default Installation (Linux):
```bash
curl -f https://getinstachip.com/install.sh | sh
```

### Default Installation (MacOS)
Download and run the `.pkg` file matching your MacOS architecture.

### Default Installation (Windows):
Download and run the `.zip` file matching your Windows architecture.


If installation doesn't work, try the following:

### Linux alternative:
We support Snap

```bash
snap download instachip-vpm
alias vpm='instachip-vpm.vpm'
```

### MacOS alternative:
```bash
brew tap getinstachip/vpm
brew install vpm
```

After installation, the vpm command will be available in any terminal.

## Commands

- `vpm docs <module.v>`: Generate documentation for any module (highlighting bugs and edge cases)
- `vpm install <tool>`: Auto-integrate an open-source tool without manual setup
- `vpm update <module.v>`: Update module to a more recent version
- `vpm restructure <module.v>`:
- `vpm remove <module.v>`: Remove a module from your project
- `vpm sim <module.sv> <testbench.sv>`: Simulate Verilog module using iverilog
  
### vpm docs
Generate comprehensive documentation for a module.

This command generates a Markdown README file containing:
- Overview and module description
- Pinout diagram
- Table of ports
- Table of parameters
- Important implementation details
- Simulation output and GTKWave waveform details (Coming soon!)
- List of any major bugs or caveats if they exist

```bash
vpm docs <MODULE.sv> [--from_repo] [--offline]
```

`<MODULE>`: Name of the module to generate documentation for. Include the file extension.

`[--from_repo]`: Optional flag to treat the module path as a link to a .v or .sv file in a GitHub repository. If not set, the path will be treated as a local file path.

`[--offline]`: Optional flag to generate documentation in offline mode for code security.

Examples:
```bash
vpm docs pfcache.v --offline
vpm docs https://github.com/ZipCPU/zipcpu/pfcache.v --from_repo
```

### vpm install
Install and set up an open-source tool for integration into your project.

This command:
- Downloads the specified tool
- Configures the tool for your system
- Integrates it with your VPM project setup

```bash
vpm install <TOOL_NAME>
```
`<TOOL_NAME>`: Name of the tool to install

Example:
```bash
vpm install verilator
```

Currently supported tools:
- Verilator
- Chipyard
- OpenROAD
- Edalize
- Icarus Verilog

Coming soon:
- Yosys (with support for ABC)
- RISC-V GNU Toolchain

### vpm update
Update a package to the latest version.

This command:
- Checks for the latest version of the specified module
- Downloads and replaces the current version with the latest
- Optionally updates to a specific version
- Optionally Updates all dependencies and submodules
- Modifies the vpm.toml file to reflect the changes

```bash
vpm update <MODULE_PATH> [--version <VERSION>]
```

`<PACKAGE_PATH>`: Full module path of the module to update

`[--version <VERSION>]`: Optional flag to update to a specific version. If not set and the module is from a git repo, the latest commit hash will be used.

Example:
```bash
vpm update vpm_modules/counter/rtl/counter.v
```

### vpm restructure
Restructure your project into the vpm_modules directory.

This command:
- Moves the top module and all submodules to its specific `vpm_modules` subdirectory
- Updates the vpm.toml file to reflect the changes

```bash
vpm restructure <TOP_MODULE_PATH>
```

`<TOP_MODULE_PATH>`: Full module path of the top module to restructure around.


Example:
```bash
vpm restructure vpm_modules/counter/rtl/counter.v
```

Note: `vpm restructure` can be used to add submodules files to the vpm_modules directory after update. Just run `vpm restructure <TOP_MODULE_PATH>` after updating the top module or any submodules and you will be prompted to add any new submodules to the directory.

### vpm remove
Remove a package from your project.

This command:
- Removes the specified module from your project
- Updates the vpm.toml file to remove the module entry
- Cleans up any orphaned dependencies

```bash
vpm remove <PACKAGE_PATH>
```

`<PACKAGE_PATH>`: Full module path of the package to remove

Example:
```bash
vpm remove vpm_modules/counter/rtl/counter.v
```

### vpm sim
Simulate Verilog files.

This command:
- Compiles the specified Verilog files
- Runs the simulation
- Provides output and analysis of the simulation results

```bash
vpm sim <VERILOG_FILES>...
```
`<VERILOG_FILES>`: List of Verilog files to simulate using Icarus Verilog.

Example:
```bash
vpm sim testbench.v module1.v module2.v
```

## Configuration

VPM uses a `vpm.toml` file for project configuration. This file allows you to specify project properties, dependencies, and custom settings.

Example vpm.toml file:
```toml
[library]
name = "my_cpu"
version = "0.3.5"
description = "A basic CPU."

[dependencies]
"https://github.com/ZipCPU/zipcpu" = [{top_module = "pfcache.v", version = "commit_hash"}, ...]
"ARM Module" = [{top_module = "arm.v", version = "0.0.1"}, ...]
```