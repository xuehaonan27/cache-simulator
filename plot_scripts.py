import os
import re
import pandas as pd
import matplotlib.pyplot as plt

# 定义一个函数来解析文件内容


def parse_file_group1(file_path):
    with open(file_path, 'r') as file:
        content = file.read()

    # 使用正则表达式提取L1 Cache的相关信息
    l1_cache_info = re.search(
        r'---------- L1 Cache ----------.*?Size = (\d+).*?Block size = (\d+).*?Miss rate: ([\d.]+)', content, re.DOTALL)

    if l1_cache_info:
        size = int(l1_cache_info.group(1))
        block_size = int(l1_cache_info.group(2))
        miss_rate = float(l1_cache_info.group(3))

        return size, block_size, miss_rate
    else:
        return None

# 定义一个函数来读取所有结果文件并解析


def read_all_files_group1(directory):
    data = []
    for filename in os.listdir(directory):
        if filename.endswith('.txt'):  # result is .txt
            file_path = os.path.join(directory, filename)
            result = parse_file_group1(file_path)
            if result:
                data.append(result)
    data.sort(key=lambda x: x[1])  # sort according to block size
    return data

# plot the figure


def plot_miss_rate_group1(data, save_path):
    df = pd.DataFrame(data, columns=['Size', 'Block Size', 'Miss Rate'])

    # Group by Size
    # Miss Rate to Block Size
    for size, group in df.groupby('Size'):
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
        size = f"{int(size)}{radix}"
        plt.plot(group['Block Size'], group['Miss Rate'],
                 marker='o', label=f'Size = {size}')

    plt.xlabel('Block Size')
    plt.ylabel('Miss Rate')
    plt.title('L1 Cache Miss Rate vs Block Size for Different Sizes')
    plt.legend()
    plt.grid(True)
    plt.savefig(save_path)
    plt.show()


def parse_file_group2(file_path):
    with open(file_path, 'r') as file:
        content = file.read()

    # 使用正则表达式提取L1 Cache, L2 Cache, L3 Cache等的相关信息
    cache_info = re.findall(
        r'---------- (L\d+ Cache) ----------.*?Associativity = (\d+).*?Miss rate: ([\d.]+)', content, re.DOTALL)

    data = []
    for cache, associativity, miss_rate in cache_info:
        data.append((cache, int(associativity), float(miss_rate)))

    return data


def read_all_files_group2(directory):
    data = []
    for filename in os.listdir(directory):
        if filename.endswith('.txt'):  # 假设结果文件是txt格式
            file_path = os.path.join(directory, filename)
            results = parse_file_group2(file_path)
            if results:
                data.extend(results)
    data.sort(key=lambda x: x[1])  # sort according to associativity
    return data


def plot_miss_rate_group2(data, save_path):

    df = pd.DataFrame(data, columns=['Cache', 'Associativity', 'Miss Rate'])

    # 按Cache分组，绘制Miss Rate随Associativity变化的折线图
    for cache, group in df.groupby('Cache'):
        plt.plot(group['Associativity'], group['Miss Rate'],
                 marker='o', label=cache)

    plt.xlabel('Associativity')
    plt.ylabel('Miss Rate')
    plt.title('Miss Rate vs Associativity for Different Caches')
    plt.legend()
    plt.grid(True)
    plt.savefig(save_path)
    plt.show()


def main():
    directory = 'sim_results/group_2/trace1/'
    save_path = 'figures/group_2/miss_rate_to_associativity_trace1.png'

    # data = read_all_files_group1(directory)
    # plot_miss_rate_group1(data, save_path)
    data = read_all_files_group2(directory)
    plot_miss_rate_group2(data, save_path)


if __name__ == "__main__":
    main()
