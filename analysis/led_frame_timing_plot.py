#!/usr/bin/env python3
"""LED Latency Frame Timing Analysis

Plots time deltas between consecutive frames to understand frame timing behavior.
"""

import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np

parser = argparse.ArgumentParser(description='Visualize LED latency frame timing')
parser.add_argument('input_csv', help='Path to row means CSV file')
parser.add_argument('--output', default='led_frame_timing_plot.png', help='Output plot filename')
parser.add_argument('--no-show', action='store_true', help='Do not display plot interactively')
args = parser.parse_args()

# Load CSV data
df = pd.read_csv(args.input_csv)

print(f"Loaded {len(df)} frames")

# Calculate time deltas between consecutive frames
time_deltas = df['time_s'].diff()
time_deltas_ms = time_deltas * 1000  # Convert to milliseconds

# Create figure
fig, axes = plt.subplots(3, 1, figsize=(14, 10))
fig.suptitle('LED Latency Test - Frame Timing Analysis', fontsize=16, fontweight='bold')

# 1. Time deltas vs frame number
ax1 = axes[0]
ax1.plot(df['frame_num'][1:], time_deltas_ms[1:], 'o-', markersize=4, linewidth=1, color='steelblue')
ax1.set_xlabel('Frame Number', fontsize=12)
ax1.set_ylabel('Time Delta (ms)', fontsize=12)
ax1.set_title('Frame-to-Frame Time Delta', fontsize=13, fontweight='bold')
ax1.grid(True, alpha=0.3)

# Mark LED states with background colors
for idx, row in df[1:].iterrows():
    led_state = row['led_state']
    color = 'gold' if led_state == 'on' else 'lightblue'
    ax1.axvspan(idx - 0.5, idx + 0.5, alpha=0.1, color=color)

# 2. Histogram of time deltas
ax2 = axes[1]
ax2.hist(time_deltas_ms[1:], bins=30, edgecolor='black', color='steelblue', alpha=0.7)
ax2.set_xlabel('Time Delta (ms)', fontsize=12)
ax2.set_ylabel('Count', fontsize=12)
ax2.set_title('Distribution of Frame Time Deltas', fontsize=13, fontweight='bold')
ax2.grid(True, alpha=0.3, axis='y')

# Add statistics to histogram
mean_delta = time_deltas_ms[1:].mean()
std_delta = time_deltas_ms[1:].std()
min_delta = time_deltas_ms[1:].min()
max_delta = time_deltas_ms[1:].max()
ax2.axvline(mean_delta, color='red', linestyle='--', linewidth=2, label=f'Mean: {mean_delta:.2f} ms')
ax2.legend()

# 3. Cumulative time vs frame number
ax3 = axes[2]
ax3.plot(df['frame_num'], df['time_s'], 'o-', markersize=4, linewidth=1, color='darkgreen')
ax3.set_xlabel('Frame Number', fontsize=12)
ax3.set_ylabel('Cumulative Time (s)', fontsize=12)
ax3.set_title('Cumulative Time vs Frame Number', fontsize=13, fontweight='bold')
ax3.grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig(args.output, dpi=150, bbox_inches='tight')
print(f"\nSaved visualization to: {args.output}")

# Print statistics
print("\n=== Frame Timing Statistics ===")
print(f"Total frames: {len(df)}")
print(f"Total time: {df['time_s'].iloc[-1]:.3f} s")
print(f"\nFrame-to-frame deltas:")
print(f"  Mean:   {mean_delta:.3f} ms")
print(f"  Std:    {std_delta:.3f} ms")
print(f"  Min:    {min_delta:.3f} ms")
print(f"  Max:    {max_delta:.3f} ms")
print(f"  Range:  {max_delta - min_delta:.3f} ms")

# Show first few deltas
print(f"\nFirst 10 frame deltas (ms):")
for i in range(1, min(11, len(df))):
    delta_ms = time_deltas_ms.iloc[i]
    led_state = df['led_state'].iloc[i]
    print(f"  Frame {i} ({led_state:3s}): {delta_ms:.3f} ms")

# Show plot interactively unless --no-show is passed
if not args.no_show:
    plt.show()
