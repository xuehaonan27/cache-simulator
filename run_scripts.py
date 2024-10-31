import os

CONFIGS_DIR_PREFIX = 'configs/group_2/'
RESULTS_DIR_PREFIX = 'sim_results/group_2/'
TRACE_DIR_PREFIX = 'trace/'
SIM = "target/release/cache-simulator"
CARGO = "cargo"

config_names = os.listdir(CONFIGS_DIR_PREFIX)
config_files = [f"{CONFIGS_DIR_PREFIX}{file}" for file in config_names]

trace_names = os.listdir(TRACE_DIR_PREFIX)
trace_files = [f"{TRACE_DIR_PREFIX}{file}" for file in trace_names]

# Compile
os.system(f"{CARGO} build --release")

# Execution
for (config_name, config_file) in zip(config_names, config_files):
    for (trace_name, trace_file) in zip(trace_names, trace_files):
        config_name = config_name.split('.')[0]
        trace_name = trace_name.split('.')[0]
        res_dir = f"{RESULTS_DIR_PREFIX}{trace_name}/"
        os.makedirs(res_dir, exist_ok=True)
        save_path = f"{res_dir}{config_name}.txt"
        os.system(
            f"{SIM} --config {config_file} --trace {trace_file} > {save_path}")
