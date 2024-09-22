#!/bin/bash

# Exit immediately if a command exits with a non-zero status
# but we'll handle errors manually to provide detailed reports
#set -e

# Function to run a command and handle error reporting
run_command() {
    local description="$1"
    shift
    local cmd=("$@")

    echo "=== Testing: $description ==="
    echo "Command: ${cmd[@]}"

    # Run the command, capturing stdout and stderr
    output=$( "${cmd[@]}" 2>&1 )
    exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo "✅ Success"
    else
        echo "❌ Failure (Exit Code: $exit_code)"
        echo "Error Output:"
        echo "$output"
    fi
    echo "------------------------------"
}

# Ensure we're in the project directory
cd "$(dirname "$0")" || { echo "Failed to navigate to script directory."; exit 1; }

# Initialize a temporary directory for testing (optional)
# Uncomment the following lines if you want to run tests in a temporary environment
mkdir -p temp_test_env
cd temp_test_env || { echo "Failed to create temporary test environment."; exit 1; }

# 1. Test `include` command
run_command "Include a module as a single module" \
    cargo run include "https://github.com/ZipCPU/zipcpu/blob/master/rtl/core/pfcache.v"

run_command "Include a module as a single module - riscv" \
    cargo run include "https://github.com/ultraembedded/riscv/blob/master/core/riscv/riscv_alu.v" --riscv

run_command "Include a repository" \
    cargo run include --repo "ZipCPU/zipcpu"

run_command "Include with specific commit and documentation" \
    cargo run include "https://github.com/ZipCPU/zipcpu/blob/master/rtl/core/dcache.v" --commit "136697cb2922a1f0b42d0071064a18c8ab4df451" --with_docs

run_command "Include with offline documentation" \
    cargo run include "https://github.com/ZipCPU/zipcpu/blob/master/rtl/core/axilcache.v" --with_docs --offline

# 2. Test `update` command
run_command "Update a module to latest version" \
    cargo run update "vpm_modules/dcache/rtl/dcache.v"

run_command "Update a module to a specific commit" \
    cargo run update "vpm_modules/dcache/rtl/dcache.v" --commit "136697cb2922a1f0b42d0071064a18c8ab4df451"

# 3. Test `remove` command
run_command "Remove a package from the project" \
    cargo run remove "vpm_modules/axilcache/rtl/axilcache.v"

# 4. Test `docs` command
run_command "Generate documentation for a local module" \
    cargo run docs "vpm_modules/pfcache/rtl/pfcache.v"

run_command "Generate documentation from repository with offline mode" \
    cargo run docs "vpm_modules/riscv_alu/rtl/riscv_alu.v" --from_repo --offline

# 5. Test `install` command
run_command "Install Verilator tool" \
    cargo run install "verilator"

run_command "Install RISC-V GNU toolchain" \
    cargo run install "riscv-gnu-toolchain"

run_command "Install Icarus Verilog" \
    cargo run install "iverilog"

# 6. Test `list` command
run_command "List all available modules in the project" \
    cargo run list

# 7. Test `sim` command
run_command "Simulate Verilog files without waveform" \
    cargo run sim "vpm_modules/pfcache/rtl/pfcache.v"

run_command "Simulate Verilog files with waveform" \
    cargo run sim "vpm_modules/pfcache/rtl/pfcache.v" --waveform

# 8. Test `synth` command
run_command "Synthesize a top module for board-agnostic" \
    cargo run synth "path/to/top_module.v"

run_command "Synthesize a top module for RISC-V with core path" \
    cargo run synth "path/to/top_module.v" --riscv --core_path "path/to/core"

run_command "Synthesize a top module with Yosys script generation" \
    cargo run synth "path/to/top_module.v" --gen_yosys_script

# 9. Test `load` command
# run_command "Load a top module onto target device" \
#     cargo run load "path/to/netlist.bit" "path/to/constraints.xcd"

# run_command "Load a top module with RISC-V toolchain" \
#     cargo run load "path/to/netlist.bit" "path/to/constraints.xcd" --riscv

# 10. Test `run` command
# run_command "Run a specified program" \
#     cargo run run "path/to/program.sh"

# run_command "Run a RISC-V compiled program" \
#     cargo run run "path/to/riscv_program" --riscv

# 11. Test `upgrade` command
run_command "Upgrade VPM to the latest version" \
    cargo run upgrade

# 12. Test `config` command
run_command "Enable analytics in VPM configuration" \
    cargo run config --analytics true

run_command "Disable analytics in VPM configuration" \
    cargo run config --analytics false

# 13. Test `test` command
run_command "Test a module" \
    cargo run test "vpm_modules/pfcache/rtl/pfcache.v"

# Summary
echo "=== Testing Completed ==="
