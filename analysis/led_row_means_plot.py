#!/usr/bin/env python3
"""LED Latency Row Means Visualization

Plots row-by-row brightness deltas for LED latency test.
Shows which rows detect LED changes and when.
"""

import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np

parser = argparse.ArgumentParser(description='Visualize LED latency row means data')
parser.add_argument('input_csv', help='Path to row means CSV file')
parser.add_argument('--output', default='led_row_means_plot.png', help='Output plot filename')
parser.add_argument('--num-frames', type=int, default=None, help='Number of frames to plot (default: all)')
parser.add_argument('--no-show', action='store_true', help='Do not display plot interactively')
args = parser.parse_args()

# Load CSV data
df = pd.read_csv(args.input_csv)

# Get row column names
row_cols = [col for col in df.columns if col.startswith('row_')]
num_rows = len(row_cols)

print(f"Loaded {len(df)} frames with {num_rows} rows each")
print(f"LED states: {df['led_state'].value_counts().to_dict()}")

# Limit to first N frames for visualization
if args.num_frames is None:
    num_frames_to_plot = len(df)
    df_subset = df
else:
    num_frames_to_plot = min(args.num_frames, len(df))
    df_subset = df.head(num_frames_to_plot)

# Create figure
fig, axes = plt.subplots(2, 1, figsize=(14, 10))
if args.num_frames is None:
    title = f'LED Latency Test - Row Brightness Deltas (All {num_frames_to_plot} frames)'
else:
    title = f'LED Latency Test - Row Brightness Deltas (First {num_frames_to_plot} frames)'
fig.suptitle(title, fontsize=16, fontweight='bold')

# 1. Heatmap of row deltas over time
ax1 = axes[0]
row_data = df_subset[row_cols].values.T  # Transpose so rows are on Y-axis

im = ax1.imshow(row_data, aspect='auto', cmap='RdYlGn',
                vmin=-10, vmax=200, interpolation='nearest')
ax1.set_xlabel('Frame Number', fontsize=12)
ax1.set_ylabel('Row Number', fontsize=12)
ax1.set_title('Row Brightness Delta Heatmap', fontsize=13, fontweight='bold')

# Mark LED states on x-axis
for idx, row in df_subset.iterrows():
    led_state = row['led_state']
    color = 'gold' if led_state == 'on' else 'darkblue'
    ax1.axvline(idx, color=color, alpha=0.3, linewidth=2)

# Add colorbar
cbar = plt.colorbar(im, ax=ax1)
cbar.set_label('Brightness Delta (DN)', fontsize=11)

# Add legend for LED states
from matplotlib.patches import Patch
legend_elements = [
    Patch(facecolor='gold', alpha=0.3, label='LED ON'),
    Patch(facecolor='darkblue', alpha=0.3, label='LED OFF')
]
ax1.legend(handles=legend_elements, loc='upper right')

# 2. Line plots for each frame's row profile
ax2 = axes[1]
for idx, row in df_subset.iterrows():
    led_state = row['led_state']
    color = 'gold' if led_state == 'on' else 'darkblue'
    alpha = 0.8 if led_state == 'on' else 0.3
    linewidth = 2 if led_state == 'on' else 1

    row_values = [row[col] for col in row_cols]
    label = f"Frame {idx} ({led_state})"
    ax2.plot(range(num_rows), row_values, color=color, alpha=alpha,
             linewidth=linewidth, label=label)

ax2.set_xlabel('Row Number', fontsize=12)
ax2.set_ylabel('Brightness Delta (DN)', fontsize=12)
ax2.set_title('Row Brightness Profiles', fontsize=13, fontweight='bold')
ax2.grid(True, alpha=0.3)
ax2.axhline(0, color='red', linestyle='--', linewidth=1, alpha=0.5, label='Baseline')
ax2.legend(loc='upper right', fontsize=8, ncol=2)

plt.tight_layout()
plt.savefig(args.output, dpi=150, bbox_inches='tight')
print(f"\nSaved visualization to: {args.output}")

# Print statistics
print("\n=== Row Means Statistics ===")
for idx, row in df_subset.iterrows():
    led_state = row['led_state']
    row_values = [row[col] for col in row_cols]
    mean_val = np.mean(row_values)
    std_val = np.std(row_values)
    min_val = np.min(row_values)
    max_val = np.max(row_values)
    print(f"Frame {idx} ({led_state:3s}): mean={mean_val:6.1f} DN, std={std_val:5.2f}, range=[{min_val:6.1f}, {max_val:6.1f}]")

# Show plot interactively unless --no-show is passed
if not args.no_show:
    plt.show()
