import json, os

CONFIGS_DIR_PREFIX = 'configs/group_1/'

config = {
    "CPU_GHZ": 2.0,
    "using_latency_as_cycles": True,
    "memory": {
        "hit_latency": 100,
        "bus_latency": 0,
    },
    "caches": [
        {
            "layer": 1,
            "hit_latency": 1,
            "bus_latency": 0,
            "write_hit_policy": "WriteThrough",
            "write_miss_policy": "NonWriteAllocate",
            "replacement_policy": "LRU",
            "size": 32 * 1024,
            "associativity": 8,
            "block_size": 64,
        }
    ]
}

def save_config(config):
    config["caches"].sort(key=lambda x: x["layer"])

    cache_descs: list[str] = []
    for cache in config["caches"]:
        size = cache["size"]
        radix = ""
        if size / 1024 >= 1:
            size /= 1024
            radix = "K"
        if size / 1024 >= 1:
            size /= 1024
            radix = "M"
        if size / 1024 >= 1:
            size /= 1024
            radix = "G"
        layer = cache["layer"]
        assc = cache["associativity"]
        bs = cache["block_size"]
        cache_desc = f"L{layer}s{int(size)}{radix}w{assc}bs{bs}"
        cache_descs.append(cache_desc)

    json_path = cache_descs.pop(0)
    while True:
        try:
            path = cache_descs.pop(0)
        except Exception:
            break
        json_path += '_'
        json_path += path

    json_path += '.json'

    with open(file=f"{CONFIGS_DIR_PREFIX}{json_path}", mode='w') as f:
        json.dump(config, f, indent=4)

if __name__ == '__main__':
    for cache_size in [1, 2, 4, 8, 16, 32, 64, 128, 256]:
        cache_size *= 1024 # KiB
        config["caches"][0]["size"] = cache_size
        for bs in [8, 16, 32, 64, 128]:
            config["caches"][0]["block_size"] = bs
            save_config(config)