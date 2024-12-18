# Cache Simulator
## Introduction
A simple cache simulator written in Rust programming language.

## For Lab3-2
Run 
```
# No optimization **01-mcf-gem5-xcg**
make run config_path=./configs/group_4/before.json trace_path=./trace/trace3.txt

# No optimization **02-stream-gem5-xaa**
make run config_path=./configs/group_4/before.json trace_path=./trace/trace4.txt

# Optimization **01-mcf-gem5-xcg**
make run config_path=./configs/group_4/after.json trace_path=./trace/trace3.txt

# Optimization **02-stream-gem5-xaa**
make run config_path=./configs/group_4/after.json trace_path=./trace/trace4.txt
```

## Steps to run
**Run immediately**:
+ trace 1:
`make run config_path=./configs/test.json trace_path=./trace/trace1.txt`
+ trace 2:
`make run config_path=./configs/test.json trace_path=./trace/trace2.txt`

**Detailed Steps**:
1. Get Rust toolchain and make sure you could compile Rust codes with `cargo`.
2. Clone the repository: `git clone https://github.com/xuehaonan27/cache-simulator.git`.
3. Enter the repository: `cd cache-simulator`.
4. Prepare a configuration file as described below.
5. Prepare a trace file.
6. Run test with `make run config_path=</path/to/your/config/file> trace_path=</path/to/your/trace/file>`.
7. The results will be printed to stdout with pretty format (as least I think it's pretty).
8. Add `> /path/to/your/save/file` to save the results to a file.

## Configuration file
You can see example configuration files in `configs` directory, e.g. `test.json`.

A configuration file that could be parsed by the simulator must be a json file, containing a single object with **four required** fields:

+ CPU_GHZ: CPU frequency in Ghz.
+ using_latency_as_cycles:
  + When set to **true**: `"hit_latency": 1` or `"bus_latency": 1` means 1 cycle.
  + When set to **false**:  `"hit_latency": 1` or `"bus_latency": 1` means 1 nanosecond.
+ memory: configuration describing memory.
+ caches: a list of configuration objects describing caches. The `layer` fields must be continuous integers like 1, 2, 3, ..., which mean L1, L2, L3, ... caches.

## Raw data for Lab3-1
+ `configs/group_1` contains configuration files for question 1, and so on and so forth. These configuration files are generated by `build_scripts.py`.
+ `sim_results/group_1` contains results for question 1, and so on and so forth. These results are generated by the simulator, and the auto script is `run_scripts.py`.
+ `figures/group_1` contains pictures visualizing the results for question 1 and 2. The figures are plotted by with `plot_scripts.py`.
+ `L1s64Kw8bs64_L2s512Kw8bs64` means:
  + `L1` cache total `s`ize `64K`, `w`ays `8` (associativity), `b`lock `s`ize `64`B.
  + `L2` cache total `s`ize `512K`, `w`ays `8` (associativity), `b`lock `s`ize `64`B.
