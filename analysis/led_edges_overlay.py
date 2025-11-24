#!/usr/bin/env python3
"""LED Edge Overlay Analysis

Plots rising and falling edges overlaid for all frames to visualize
rolling shutter timing consistency.
"""

import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np

parser = argparse.ArgumentParser(description='Analyze LED latency test data with edge overlay')
parser.add_argument('input_csv', help='Path to input CSV file')
parser.add_argument('--output', default='led_edges_overlay.png', help='Output plot filename')
parser.add_argument('--pulse-delay-us', type=float, default=10000, help='Delay before LED pulse (μs)')
parser.add_argument('--led-duration-us', type=float, default=5000, help='LED on duration (μs)')
args = parser.parse_args()

df = pd.read_csv(args.input_csv)

# Filter valid frames (both edges detected)
valid = df[(df['rising_edge_row'] >= 0) & (df['falling_edge_row'] > 0)]

fig, axes = plt.subplots(2, 2, figsize=(14, 10))
fig.suptitle('LED Rolling Shutter Edge Analysis (1000 frames)', fontsize=16, fontweight='bold')

# 1. Rising and falling edges overlaid
ax1 = axes[0, 0]
ax1.scatter(valid['frame_num'], valid['rising_edge_row'], alpha=0.5, s=10, label='Rising edge', color='green')
ax1.scatter(valid['frame_num'], valid['falling_edge_row'], alpha=0.5, s=10, label='Falling edge', color='red')
ax1.set_xlabel('Frame Number', fontsize=11)
ax1.set_ylabel('Row Number', fontsize=11)
ax1.set_title('Edge Rows Over Time', fontsize=12, fontweight='bold')
ax1.legend()
ax1.grid(True, alpha=0.3)

# 2. Histogram of rising edges
ax2 = axes[0, 1]
ax2.hist(valid['rising_edge_row'], bins=50, edgecolor='black', alpha=0.7, color='green')
mean_rise = valid['rising_edge_row'].mean()
std_rise = valid['rising_edge_row'].std()
ax2.axvline(mean_rise, color='darkgreen', linestyle='--', linewidth=2,
            label=f'Mean: {mean_rise:.1f} ± {std_rise:.1f}')
ax2.set_xlabel('Row Number', fontsize=11)
ax2.set_ylabel('Count', fontsize=11)
ax2.set_title('Rising Edge Row Distribution', fontsize=12, fontweight='bold')
ax2.legend()
ax2.grid(True, alpha=0.3)

# 3. Line rate histogram
ax3 = axes[1, 0]
line_rates = valid['line_rate_us']
ax3.hist(line_rates, bins=50, edgecolor='black', alpha=0.7, color='steelblue')
mean_rate = line_rates.mean()
std_rate = line_rates.std()
ax3.axvline(mean_rate, color='red', linestyle='--', linewidth=2,
            label=f'Mean: {mean_rate:.3f} ± {std_rate:.3f} μs/row')
ax3.set_xlabel('Line Rate (μs/row)', fontsize=11)
ax3.set_ylabel('Count', fontsize=11)
ax3.set_title('Line Rate Distribution', fontsize=12, fontweight='bold')
ax3.legend()
ax3.grid(True, alpha=0.3)

# 4. Frame timing jitter - compare both timers
ax4 = axes[1, 1]
delta_ms = valid['delta_ms'].iloc[1:]  # Skip first frame
if 'delta_ms_instant' in valid.columns:
    delta_ms_instant = valid['delta_ms_instant'].iloc[1:]
    ax4.hist(delta_ms, bins=50, alpha=0.7, color='purple', label='V4L2 timestamp')
    ax4.hist(delta_ms_instant, bins=50, alpha=0.5, color='orange', label='Instant timer')
    mean_delta = delta_ms.mean()
    std_delta = delta_ms.std()
    mean_instant = delta_ms_instant.mean()
    std_instant = delta_ms_instant.std()
    ax4.axvline(mean_delta, color='purple', linestyle='--', linewidth=2)
    ax4.axvline(mean_instant, color='orange', linestyle='--', linewidth=2)
    ax4.legend(title=f'V4L2: {mean_delta:.6f}±{std_delta:.6f}\nInstant: {mean_instant:.6f}±{std_instant:.6f}')
else:
    ax4.hist(delta_ms, bins=50, edgecolor='black', alpha=0.7, color='purple')
    mean_delta = delta_ms.mean()
    std_delta = delta_ms.std()
    ax4.axvline(mean_delta, color='red', linestyle='--', linewidth=2,
                label=f'Mean: {mean_delta:.6f} ± {std_delta:.6f} ms')
    ax4.legend()
ax4.set_xlabel('Inter-frame Time (ms)', fontsize=11)
ax4.set_ylabel('Count', fontsize=11)
ax4.set_title('Frame Period Distribution', fontsize=12, fontweight='bold')
ax4.grid(True, alpha=0.3)

plt.tight_layout()
plt.savefig(args.output, dpi=150, bbox_inches='tight')
print(f"Saved visualization to: {args.output}")

# Second figure: Edge timing analysis
fig2, ax = plt.subplots(figsize=(12, 8))
fig2.suptitle('LED Edge Timing Analysis', fontsize=16, fontweight='bold')

# Convert row numbers to time using mean line rate
rising_time_us = valid['rising_edge_row'] * mean_rate
falling_time_us = valid['falling_edge_row'] * mean_rate

# Plot rising and falling edges
ax.scatter(rising_time_us, valid['rising_edge_row'], alpha=0.3, s=15, color='green', label='Rising edge')
ax.scatter(falling_time_us, valid['falling_edge_row'], alpha=0.3, s=15, color='red', label='Falling edge')

# Regression lines
from numpy.polynomial import polynomial as P
rise_coeffs = P.polyfit(rising_time_us, valid['rising_edge_row'], 1)
fall_coeffs = P.polyfit(falling_time_us, valid['falling_edge_row'], 1)

rise_fit_x = np.array([rising_time_us.min(), rising_time_us.max()])
rise_fit_y = P.polyval(rise_fit_x, rise_coeffs)
fall_fit_x = np.array([falling_time_us.min(), falling_time_us.max()])
fall_fit_y = P.polyval(fall_fit_x, fall_coeffs)

ax.plot(rise_fit_x, rise_fit_y, 'g--', linewidth=2, label=f'Rising fit')
ax.plot(fall_fit_x, fall_fit_y, 'r--', linewidth=2, label=f'Falling fit')

# Programmed delay line
ax.axvline(args.pulse_delay_us, color='blue', linestyle='-', linewidth=2,
           label=f'Programmed delay: {args.pulse_delay_us} μs')

# Calculate intercepts (row=0 intercept gives time when edge would hit row 0)
# For row = m*time + b, time at row=0 is -b/m
rise_slope, rise_intercept = rise_coeffs[1], rise_coeffs[0]
fall_slope, fall_intercept = fall_coeffs[1], fall_coeffs[0]

# Time when edge would hit row 0
rise_time_at_row0 = -rise_intercept / rise_slope if rise_slope != 0 else 0
fall_time_at_row0 = -fall_intercept / fall_slope if fall_slope != 0 else 0

ax.axvline(rise_time_at_row0, color='darkgreen', linestyle=':', linewidth=2,
           label=f'Rising intercept: {rise_time_at_row0:.1f} μs')
ax.axvline(fall_time_at_row0, color='darkred', linestyle=':', linewidth=2,
           label=f'Falling intercept: {fall_time_at_row0:.1f} μs')

# Calculate and display differences
delay_to_rise = args.pulse_delay_us - rise_time_at_row0
delay_to_fall = fall_time_at_row0 - rise_time_at_row0
actual_duration = delay_to_fall

ax.set_xlabel('Time (μs)', fontsize=12)
ax.set_ylabel('Row Number', fontsize=12)
ax.legend(loc='upper left', fontsize=9)
ax.grid(True, alpha=0.3)

# Add text box with timing analysis
textstr = '\n'.join([
    f'Programmed delay: {args.pulse_delay_us} μs',
    f'Programmed duration: {args.led_duration_us} μs',
    f'',
    f'Rising edge intercept: {rise_time_at_row0:.1f} μs',
    f'Falling edge intercept: {fall_time_at_row0:.1f} μs',
    f'',
    f'Actual latency (delay - rise): {delay_to_rise:.1f} μs',
    f'Actual duration (fall - rise): {actual_duration:.1f} μs',
    f'Duration error: {actual_duration - args.led_duration_us:.1f} μs'
])
props = dict(boxstyle='round', facecolor='wheat', alpha=0.8)
ax.text(0.98, 0.02, textstr, transform=ax.transAxes, fontsize=10,
        verticalalignment='bottom', horizontalalignment='right', bbox=props)

plt.tight_layout()
output2 = args.output.replace('.png', '_timing.png')
plt.savefig(output2, dpi=150, bbox_inches='tight')
print(f"Saved timing analysis to: {output2}")

# Third figure: Pulse duration from edge difference
fig3, ax3_new = plt.subplots(figsize=(10, 6))
fig3.suptitle('LED Pulse Duration from Edge Difference', fontsize=16, fontweight='bold')

# Calculate pulse duration using line rate
bright_rows = valid['falling_edge_row'] - valid['rising_edge_row']
pulse_duration_us = bright_rows * valid['line_rate_us']

ax3_new.scatter(valid['frame_num'], pulse_duration_us, alpha=0.5, s=10, color='purple')
mean_duration = pulse_duration_us.mean()
std_duration = pulse_duration_us.std()

ax3_new.axhline(mean_duration, color='red', linestyle='--', linewidth=2,
                label=f'Mean: {mean_duration:.2f} ± {std_duration:.2f} μs')
ax3_new.axhline(args.led_duration_us, color='blue', linestyle='-', linewidth=2,
                label=f'Programmed: {args.led_duration_us} μs')

ax3_new.set_xlabel('Frame Number', fontsize=12)
ax3_new.set_ylabel('Pulse Duration (μs)', fontsize=12)
ax3_new.legend()
ax3_new.grid(True, alpha=0.3)

# Add text box
error_us = mean_duration - args.led_duration_us
textstr = '\n'.join([
    f'Programmed duration: {args.led_duration_us} μs',
    f'Measured duration: {mean_duration:.2f} ± {std_duration:.2f} μs',
    f'Error: {error_us:+.2f} μs ({100*error_us/args.led_duration_us:+.2f}%)'
])
props = dict(boxstyle='round', facecolor='wheat', alpha=0.8)
ax3_new.text(0.98, 0.02, textstr, transform=ax3_new.transAxes, fontsize=10,
             verticalalignment='bottom', horizontalalignment='right', bbox=props)

plt.tight_layout()
output3 = args.output.replace('.png', '_duration.png')
plt.savefig(output3, dpi=150, bbox_inches='tight')
print(f"Saved duration analysis to: {output3}")

# Show all figures at once
plt.show()

# Calculate latency from exposure end to frame available in RAM
# The rising edge row tells us when the pulse hit the sensor relative to frame start
# Pulse fired at: pulse_delay_us after previous frame callback
# Rising edge at row N means pulse arrived at N * line_rate_us into the exposure
# Latency = pulse_delay_us - (rising_edge_row * line_rate_us)
# This is the time from exposure end of previous frame to when that frame was available

pulse_arrival_us = valid['rising_edge_row'] * mean_rate
latency_us = args.pulse_delay_us - pulse_arrival_us
mean_latency = latency_us.mean()
std_latency = latency_us.std()

# Print summary
print("\n=== LED Latency Test Summary ===")
print(f"\nValid frames: {len(valid)}/{len(df)}")
print(f"\nLine Rate:")
print(f"  Mean: {mean_rate:.3f} μs/row")
print(f"  Std:  {std_rate:.3f} μs/row")
print(f"\nFrame Period:")
print(f"  Mean: {mean_delta:.6f} ms ({1000/mean_delta:.1f} FPS)")
print(f"  Std:  {std_delta:.6f} ms")
print(f"\nRising Edge Row:")
print(f"  Mean: {mean_rise:.1f}")
print(f"  Std:  {std_rise:.1f}")
print(f"  Range: {valid['rising_edge_row'].min()}-{valid['rising_edge_row'].max()}")
print(f"\nBright Rows:")
print(f"  Mean: {valid['bright_rows'].mean():.1f}")
print(f"  Range: {valid['bright_rows'].min()}-{valid['bright_rows'].max()}")
print(f"\nFrame Readout Latency (exposure end to RAM available):")
print(f"  Pulse delay: {args.pulse_delay_us:.0f} μs")
print(f"  Pulse arrival (row * line_rate): {pulse_arrival_us.mean():.1f} ± {pulse_arrival_us.std():.1f} μs")
print(f"  Latency: {mean_latency:.1f} ± {std_latency:.1f} μs ({mean_latency/1000:.2f} ms)")
