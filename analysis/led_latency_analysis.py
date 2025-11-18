#!/usr/bin/env python3
"""LED Latency Test Analysis

Analyzes rolling shutter timing data from LED toggle experiments.
Generates visualizations for:
- Inter-frame timing jitter (histogram)
- Row detection distribution
- Brightness delta stability
"""

import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np

# Parse command line arguments
parser = argparse.ArgumentParser(description='Analyze LED latency test data')
parser.add_argument('input_csv', help='Path to input CSV file')
parser.add_argument('--output', default='led_latency_analysis.png', help='Output plot filename')
args = parser.parse_args()

# Load CSV data
df = pd.read_csv(args.input_csv)

# Calculate inter-frame time differences
time_diffs = df['time_s'].diff().dropna() * 1000  # Convert to milliseconds

# Create figure with subplots
fig, axes = plt.subplots(2, 2, figsize=(14, 10))
fig.suptitle('LED Latency Test - Rolling Shutter Characterization', fontsize=16, fontweight='bold')

# 1. Inter-frame timing histogram
ax1 = axes[0, 0]
ax1.hist(time_diffs, bins=30, edgecolor='black', alpha=0.7, color='steelblue')
ax1.axvline(time_diffs.mean(), color='red', linestyle='--', linewidth=2, label=f'Mean: {time_diffs.mean():.3f} ms')
ax1.axvline(time_diffs.median(), color='orange', linestyle='--', linewidth=2, label=f'Median: {time_diffs.median():.3f} ms')
ax1.set_xlabel('Inter-frame Time (ms)', fontsize=11)
ax1.set_ylabel('Count', fontsize=11)
ax1.set_title('Frame Timing Jitter Distribution', fontsize=12, fontweight='bold')
ax1.legend()
ax1.grid(True, alpha=0.3)

# Add statistics text
stats_text = f'Std Dev: {time_diffs.std():.4f} ms\nMin: {time_diffs.min():.3f} ms\nMax: {time_diffs.max():.3f} ms'
ax1.text(0.98, 0.97, stats_text, transform=ax1.transAxes, fontsize=9,
         verticalalignment='top', horizontalalignment='right',
         bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))

# 2. Row detection distribution (LED-ON frames only)
ax2 = axes[0, 1]
led_on_frames = df[df['led_state'] == 'on']
ax2.hist(led_on_frames['row_num'], bins=20, edgecolor='black', alpha=0.7, color='forestgreen')
ax2.set_xlabel('Row Number', fontsize=11)
ax2.set_ylabel('Count', fontsize=11)
ax2.set_title('LED Detection Row Distribution (LED-ON frames)', fontsize=12, fontweight='bold')
ax2.grid(True, alpha=0.3)

# Add mean row line
mean_row = led_on_frames['row_num'].mean()
ax2.axvline(mean_row, color='red', linestyle='--', linewidth=2, label=f'Mean: {mean_row:.1f}')
ax2.legend()

# 3. Brightness delta over time
ax3 = axes[1, 0]
led_on = df[df['led_state'] == 'on']
led_off = df[df['led_state'] == 'off']
ax3.scatter(led_on['time_s'], led_on['brightness_delta'], alpha=0.6, s=30, label='LED ON', color='gold')
ax3.scatter(led_off['time_s'], led_off['brightness_delta'], alpha=0.6, s=30, label='LED OFF', color='darkblue')
ax3.set_xlabel('Time (s)', fontsize=11)
ax3.set_ylabel('Brightness Delta (DN)', fontsize=11)
ax3.set_title('Brightness Delta Stability', fontsize=12, fontweight='bold')
ax3.legend()
ax3.grid(True, alpha=0.3)

# 4. Row number vs time (shows rolling shutter timing variation)
ax4 = axes[1, 1]
ax4.scatter(led_on_frames['time_s'], led_on_frames['row_num'], alpha=0.6, s=30, color='crimson')
ax4.set_xlabel('Time (s)', fontsize=11)
ax4.set_ylabel('Detected Row Number', fontsize=11)
ax4.set_title('Row Detection vs Time (LED-ON frames)', fontsize=12, fontweight='bold')
ax4.grid(True, alpha=0.3)

# Add horizontal line at expected center
ax4.axhline(50, color='gray', linestyle=':', linewidth=1, alpha=0.5, label='ROI Center (row 50)')
ax4.legend()

plt.tight_layout()
plt.savefig(args.output, dpi=150, bbox_inches='tight')
print(f"Saved visualization to: {args.output}")

# Print summary statistics
print("\n=== LED Latency Test Summary ===")
print(f"\nFrame Timing:")
print(f"  Mean inter-frame time: {time_diffs.mean():.3f} ms")
print(f"  Std deviation: {time_diffs.std():.4f} ms")
print(f"  Jitter (max-min): {time_diffs.max() - time_diffs.min():.3f} ms")

print(f"\nLED Detection (ON frames):")
print(f"  Frames detected: {len(led_on_frames)}/{len(df[df['led_state'] == 'on'])}")
print(f"  Mean brightness delta: {led_on_frames['brightness_delta'].mean():.2f} DN")
print(f"  Std deviation: {led_on_frames['brightness_delta'].std():.2f} DN")

print(f"\nRow Distribution:")
print(f"  Mean row: {led_on_frames['row_num'].mean():.1f}")
print(f"  Row range: {led_on_frames['row_num'].min()}-{led_on_frames['row_num'].max()}")
print(f"  Std deviation: {led_on_frames['row_num'].std():.1f}")

print(f"\nBackground Noise (OFF frames):")
print(f"  Mean delta: {led_off['brightness_delta'].mean():.2f} DN")
print(f"  Max delta: {led_off['brightness_delta'].max():.2f} DN")
