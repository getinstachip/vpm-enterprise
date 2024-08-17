
# Verilog Package Manager (VPM)
[![release](https://github.com/getinstachip/vpm/actions/workflows/release.yml/badge.svg)](https://github.com/getinstachip/vpm/actions/workflows/release.yml)
![downloads](https://img.shields.io/github/downloads/getinstachip/vpm/total?logo=github&logoColor=white&style=flat-square)

VPM is a package manager for Verilog projects, being piloted at Stanford and UC Berkeley. It's designed to simplify the management of IP cores and dependencies in hardware design workflows.

## Features

- Install submodules within repositories with dependencies automatically resolved
- Automatically handle synthesis collateral including what's needed for build (COMING SOON!)
- God-tier version control with a .lock file

## Installation

To install VPM, you don't need any dependencies! Just run the following command:

```bash
curl -f https://getinstachip.com/install.sh | sh
```

After installation, you can use the `vpm` command in any terminal.

### Basic Commands

`vpm init <project_name> [version] [description] [authors] [license]`: initializes a new VPM project and create a `vpm.toml` file with the following fields
- Options:
  - `<project_name>`: Name of the project 
  - `[version]`: User-specified version of the project
  - `[description]`: Description of the project (can be enclosed by quotes)
  - `[authors]`: List of authors (`", "` separated, e.g. `"John Doe, Jane Doe"`)
  - `[license]`: License of the project (`", "` separated license-location pairs, e.g. `"MIT: <source repo #1>, Apache-2.0: <source repo #2>"`)

*Example video coming soon!*

`vpm install <module.v> <repo_url> [version]`: installs a Verilog (.v) file and all submodule dependencies from the given repoand updates the `vpm.toml` file with the new module's deatils
- Options:
  - `<module.v>`: Verilog module to install
  - `<repo_url>`: Link to the repository where the module is stored
  - `[version]`: User-specified version of the module

![vpm_install](https://github.com/user-attachments/assets/481384eb-5b71-4284-b9e3-08ea807afa34)

`vpm docs <module.v> <repo_url>`: generates a complete Markdown README documentation file for the given module 
- Options:
  - `<module.v>`: Verilog module to generate documentation for
  - `<repo_url>`: Link to the repository where the module is stored
&nbsp;
- Generation location can be overwritten in `vpm.toml`. All documentaion contains the following sections:
  1. Overview and module description
  2. Pinout diagram
  3. Table of ports
  4. Table of parameters
  5. Important implementation details
  6. Simulation output and details (Coming soon!)
  7. List of any major bugs or caveats (if they exist)

![docs](https://github.com/user-attachments/assets/9f1b9cb4-05e1-4e69-9440-16d498277f0f)

`vpm dotf <module.v>`: generates a ".f" file list for module.v and for all locally scoped defines for the submodules and links everything accordingly
- Options:
  - `<module.v>`: Local top Verilog module to generate the file list for

*Example video coming soon!*

## Configuration

Close your eyes, relax. Submodule dependencies are taken care of with our parser. Use the appropriate fields in `vpm.toml` to adjust the properties of your project. We are working on handling synthesis collateral.

Example `vpm.toml` file:

```toml
[library]
name = "library_name"
version = "0.3.5"
description = "Most important library in the world"
authors = ["First Last"]
license = [
    {type="BSD-3-Clause", source=["folder_with_artifacts/*.whatever"]},
    {type="CC-4", source=["folder_with_artifacts/*.whatever"]},
    {type="Copyright@RandomStuffyCompany", source=["whatever"]},
]
include = [
    "folder_with_modules/*",
]

[config]
configparam1=true
configparam2=false

[docs]
docspath="./not-standard-docs-path"
docsoption1=true
docsoption2=false

[dependencies]
"https://github.com/ZipCPU/zipcpu" = {"version"="1.1.1", alias="unique_library_name", modules = ["m1", "m2"], branch="not-main", commit="hash"}
"https://github.com/ZipCPU/zipcpu" = {"version"="1.1.1", alias="unique_library_name", modules = ["m1", "m2"], branch="not-main", commit="hash"}

[dev-dependencies]
"./path/to/file" = {"version"="1.1.1", alias="unique_library_name", modules = ["m1", "m2"], branch="not-main", commit="hash"}
```

- `[library]`: Contains the metadata for the library/project
  - `name`: Name of the library/project
  - `version`: Version of the library/project
  - `description`: Description of the library/project
  - `authors`: List of authors
  - `license`: List of licenses and their source locations
  - `include`: List of directories to include in the library/project
&nbsp;
- `[config]`: Contains the configuration parameters for the library/project. Custom options will be added here.
&nbsp;
- `[docs]`: Contains the documentation generation parameters for the library. Custom options will be added here.
  - `docspath`: Path to folder with all generated documentation
&nbsp;
- `[dependencies]`: Contains the external dependencies for the library/project
  - `url`: URL of the dependency repository
  - `version`: User-specified version of the dependency
  - `alias`: Alias for the dependency
  - `modules`: List of modules in the dependency, including submodule dependencies
  - `branch`: Branch of repository the dependency is on
  - `commit`: Commit hash of the repository the dependency is on
&nbsp;
- `[dev-dependencies]`: Contains the development dependencies for the library/project
  - `url`: URL of the dependency repository
  - `version`: User-specified version of the dependency
  - `alias`: Alias for the dependency
  - `modules`: List of modules in the dependency, including submodule dependencies
  - `branch`: Branch of repository the dependency is on
  - `commit`: Commit hash of the repository the dependency is on

## Enterprise version

We are receiving overwhelming interest for an enterprise version with additional features and integrations to manage internal IP for ASIC/FPGA companies.

[Join the waitlist if you're interested](https://www.waitlistr.com/lists/ce1719b7/vpm-enterprise-version-waitlist), we're launching an enterprise batch pilot soon.

## Support

For issues and feature requests, please email sathvikr@getinstachip.com.
