#!/usr/bin/env python3
import argparse
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path

parser = argparse.ArgumentParser(
    description='Analyze FGS shootout experiment results and tracking data'
)
parser.add_argument(
    '--dir', '-d',
    type=str,
    help='Path to experiment directory (default: most recent experiment_* in fgs_output/)'
)
parser.add_argument(
    '--output-dir', '-o',
    type=str,
    help='Output directory for plots (default: same as experiment directory)'
)
parser.add_argument(
    '--cutoff', '-c',
    type=float,
    default=0.5,
    help='Maximum error value to include in pixels (default: 0.5)'
)
parser.add_argument(
    '--no-show',
    action='store_true',
    help='Do not display plots interactively'
)
args = parser.parse_args()

# Find experiment directory
if args.dir:
    experiment_dir = Path(args.dir)
else:
    fgs_output_dir = Path("fgs_output")
    experiment_dirs = sorted(fgs_output_dir.glob("experiment_*"))
    if not experiment_dirs:
        raise FileNotFoundError("No experiment directories found in fgs_output/")
    experiment_dir = experiment_dirs[-1]

print(f"Analyzing experiment: {experiment_dir}")

# Set output directory (default to experiment directory)
output_dir = Path(args.output_dir) if args.output_dir else experiment_dir
output_dir.mkdir(parents=True, exist_ok=True)

# Load data files
results_file = experiment_dir / "results.csv"
tracking_file = experiment_dir / "tracking.csv"

if not results_file.exists():
    raise FileNotFoundError(f"results.csv not found in {experiment_dir}")
if not tracking_file.exists():
    raise FileNotFoundError(f"tracking.csv not found in {experiment_dir}")

print(f"Loading results from: {results_file}")
results_df = pd.read_csv(results_file)
print(f"Loading tracking data from: {tracking_file}")
tracking_df = pd.read_csv(tracking_file)

# Get sensor name from results
sensor_name = results_df['sensor'].iloc[0]
print(f"Sensor: {sensor_name}")

# ========================================
# Error Histogram Analysis (from results.csv)
# ========================================

print(f"\n=== Error Analysis ===")
results_filtered = results_df[
    (results_df['std_x_error_pixels'] <= args.cutoff) &
    (results_df['std_y_error_pixels'] <= args.cutoff)
]
print(f"Filtered from {len(results_df)} to {len(results_filtered)} rows (cutoff: {args.cutoff} pixels)")

exposure_times = sorted(results_filtered['exposure_ms'].unique())
num_exposures = len(exposure_times)
print(f"Found {num_exposures} exposure times: {exposure_times}")

# Figure 1: Error histograms
x_min, x_max = 0, args.cutoff
y_min, y_max = 0, args.cutoff

fig1, axes1 = plt.subplots(2, num_exposures, figsize=(4 * num_exposures, 8))
if num_exposures == 1:
    axes1 = axes1.reshape(2, 1)

for col_idx, exposure in enumerate(exposure_times):
    exposure_data = results_filtered[results_filtered['exposure_ms'] == exposure]

    # Top row: std_x_error_pixels
    ax_x = axes1[0, col_idx]
    color_x = 'tab:blue'
    ax_x.hist(
        exposure_data['std_x_error_pixels'],
        bins=20,
        range=(x_min, x_max),
        edgecolor='black',
        alpha=0.7,
        color=color_x
    )
    mean_x = exposure_data['std_x_error_pixels'].mean()
    ax_x.axvline(mean_x, color=color_x, linestyle='--', linewidth=2, alpha=0.5)
    ax_x.text(
        mean_x, ax_x.get_ylim()[1] * 0.95,
        f'μ={mean_x:.3f} (n={len(exposure_data)})',
        ha='center', va='top', fontsize=10,
        bbox=dict(boxstyle='round', facecolor='white', alpha=0.7)
    )
    ax_x.set_title(f'Exposure: {exposure} ms')
    ax_x.set_xlabel('std_x_error_pixels')
    ax_x.set_xlim(x_min, x_max)
    ax_x.grid(True, alpha=0.3)

    # Bottom row: std_y_error_pixels
    ax_y = axes1[1, col_idx]
    color_y = 'tab:orange'
    ax_y.hist(
        exposure_data['std_y_error_pixels'],
        bins=20,
        range=(y_min, y_max),
        edgecolor='black',
        alpha=0.7,
        color=color_y
    )
    mean_y = exposure_data['std_y_error_pixels'].mean()
    ax_y.axvline(mean_y, color=color_y, linestyle='--', linewidth=2, alpha=0.5)
    ax_y.text(
        mean_y, ax_y.get_ylim()[1] * 0.95,
        f'μ={mean_y:.3f}',
        ha='center', va='top', fontsize=10,
        bbox=dict(boxstyle='round', facecolor='white', alpha=0.7)
    )
    ax_y.set_xlabel('std_y_error_pixels')
    ax_y.set_xlim(y_min, y_max)
    ax_y.grid(True, alpha=0.3)

fig1.text(0.02, 0.75, 'X Error', rotation=90, va='center', fontsize=14, fontweight='bold')
fig1.text(0.02, 0.25, 'Y Error', rotation=90, va='center', fontsize=14, fontweight='bold')

plt.tight_layout(rect=[0.03, 0, 1, 0.96])
fig1.suptitle(f'Tracking Error Distribution - {sensor_name}', fontsize=16, fontweight='bold', y=0.98)
error_hist_output = output_dir / 'error_histograms.png'
plt.savefig(error_hist_output, dpi=150, bbox_inches='tight')
print(f"Saved error histograms to: {error_hist_output}")

# Figure 2: Magnitude vs X Error
fig2, axes2 = plt.subplots(1, num_exposures, figsize=(5 * num_exposures, 5))
if num_exposures == 1:
    axes2 = [axes2]

mag_min = results_filtered['guide_star_magnitude'].min()
mag_max = results_filtered['guide_star_magnitude'].max()

for col_idx, exposure in enumerate(exposure_times):
    exposure_data = results_filtered[results_filtered['exposure_ms'] == exposure]

    ax = axes2[col_idx]
    ax.scatter(
        exposure_data['guide_star_magnitude'],
        exposure_data['std_x_error_pixels'],
        alpha=0.5,
        s=20,
        edgecolors='black',
        linewidth=0.5
    )
    ax.set_title(f'Exposure: {exposure} ms')
    ax.set_xlabel('Guide Star Magnitude')
    ax.set_ylabel('std_x_error_pixels')
    ax.set_xlim(mag_min, mag_max)
    ax.set_ylim(0, args.cutoff)
    ax.grid(True, alpha=0.3)

plt.tight_layout(rect=[0, 0, 1, 0.96])
fig2.suptitle(f'Magnitude vs Tracking Error - {sensor_name}', fontsize=16, fontweight='bold', y=0.98)
magnitude_error_output = output_dir / 'magnitude_vs_error.png'
plt.savefig(magnitude_error_output, dpi=150, bbox_inches='tight')
print(f"Saved magnitude vs error plot to: {magnitude_error_output}")

# ========================================
# Flux Rate Analysis (from tracking.csv)
# ========================================

print(f"\n=== Flux Rate Analysis ===")
print(f"Loaded {len(tracking_df)} tracking data points")

tracking_df['flux_rate'] = tracking_df['flux'] / tracking_df['exposure_ms']

tracking_exposure_times = sorted(tracking_df['exposure_ms'].unique())
num_tracking_exposures = len(tracking_exposure_times)
print(f"Found {num_tracking_exposures} exposure times: {tracking_exposure_times}")

# HWK4123 sensor saturation parameters
bit_depth = 12
dn_per_electron = 7.42
max_well_depth_e = 7500.0

well_saturation_dn = max_well_depth_e * dn_per_electron
adc_max_dn = (2**bit_depth - 1)
saturating_reading_dn = min(well_saturation_dn, adc_max_dn)
saturating_flux_e = saturating_reading_dn / dn_per_electron

print(f"\nHWK4123 Saturation:")
print(f"  ADC max: {adc_max_dn} DN")
print(f"  Well saturation: {well_saturation_dn:.1f} DN")
print(f"  Saturating reading: {saturating_reading_dn} DN")
print(f"  Saturating flux: {saturating_flux_e:.1f} electrons\n")

# Figure 3: Flux vs Magnitude and Magnitude Histograms
fig3, axes3 = plt.subplots(2, num_tracking_exposures, figsize=(5 * num_tracking_exposures, 10))
if num_tracking_exposures == 1:
    axes3 = axes3.reshape(2, 1)

tracking_mag_min = tracking_df['magnitude'].min()
tracking_mag_max = tracking_df['magnitude'].max()
flux_min = tracking_df['flux'].min()
flux_max = tracking_df['flux'].max()

for idx, exp_time in enumerate(tracking_exposure_times):
    ax_scatter = axes3[0, idx]
    ax_hist = axes3[1, idx]
    exposure_data = tracking_df[tracking_df['exposure_ms'] == exp_time]

    mean_mag = exposure_data['magnitude'].mean()

    # Top row: Flux vs Magnitude
    ax_scatter.scatter(
        exposure_data['magnitude'],
        exposure_data['flux'],
        alpha=0.2,
        s=20,
        color='blue',
        edgecolors='none'
    )

    # Mean magnitude line
    ax_scatter.axvline(mean_mag, color='orange', linestyle='--', linewidth=2, alpha=0.8)
    ax_scatter.text(
        mean_mag, ax_scatter.get_ylim()[1] * 0.5,
        f'μ={mean_mag:.2f}',
        rotation=90, va='bottom', ha='right', fontsize=10,
        bbox=dict(boxstyle='round', facecolor='white', alpha=0.7)
    )

    ax_scatter.set_ylabel('Counts (DN)')
    ax_scatter.set_title(f'Exposure: {exp_time} ms')
    ax_scatter.grid(True, alpha=0.3)
    ax_scatter.set_yscale('log')
    ax_scatter.set_xlim(tracking_mag_min, tracking_mag_max)
    ax_scatter.set_ylim(flux_min, flux_max)

    # Bottom row: Magnitude histograms
    weights = np.ones_like(exposure_data['magnitude']) / len(exposure_data['magnitude']) * 100
    ax_hist.hist(
        exposure_data['magnitude'],
        bins=50,
        range=(tracking_mag_min, tracking_mag_max),
        edgecolor='black',
        alpha=0.7,
        color='blue',
        weights=weights
    )

    ax_hist.axvline(mean_mag, color='orange', linestyle='--', linewidth=2, alpha=0.8)
    ax_hist.text(
        mean_mag, ax_hist.get_ylim()[1] * 0.95,
        f'μ={mean_mag:.2f}',
        rotation=90, va='top', ha='right', fontsize=10,
        bbox=dict(boxstyle='round', facecolor='white', alpha=0.7)
    )

    ax_hist.set_xlabel('Stellar Magnitude (Gaia)')
    ax_hist.set_ylabel('Percentage (%)')
    ax_hist.set_xlim(tracking_mag_min, tracking_mag_max)
    ax_hist.grid(True, alpha=0.3)

plt.tight_layout(rect=[0, 0, 1, 0.96])
fig3.suptitle(f'Flux vs Magnitude - {sensor_name}', fontsize=16, fontweight='bold', y=0.98)
flux_output = output_dir / 'flux_rate_vs_magnitude.png'
plt.savefig(flux_output, dpi=150, bbox_inches='tight')
print(f"Saved flux plot to: {flux_output}")

# ========================================
# Error vs Flux Analysis
# ========================================

print(f"\n=== Error vs Flux Analysis ===")

# Calculate average flux per experiment from tracking data
avg_flux_per_experiment = tracking_df.groupby('experiment_id')['flux'].mean().reset_index()
avg_flux_per_experiment.columns = ['experiment_id', 'avg_flux']

# Merge with results data
results_with_flux = results_df.merge(avg_flux_per_experiment, on='experiment_id', how='left')

# Filter to only include experiments that had successful tracking
results_with_flux = results_with_flux.dropna(subset=['avg_flux'])

print(f"Merged {len(results_with_flux)} experiments with flux data")

# Figure 4: std_x_error vs average flux
exposure_times_flux = sorted(results_with_flux['exposure_ms'].unique())
num_exposures_flux = len(exposure_times_flux)

fig4, axes4 = plt.subplots(1, num_exposures_flux, figsize=(5 * num_exposures_flux, 5))
if num_exposures_flux == 1:
    axes4 = [axes4]

# Calculate global flux limits
flux_min_global = results_with_flux['avg_flux'].min()
flux_max_global = results_with_flux['avg_flux'].max()

for idx, exp_time in enumerate(exposure_times_flux):
    ax = axes4[idx]
    exposure_data = results_with_flux[results_with_flux['exposure_ms'] == exp_time]

    ax.scatter(
        exposure_data['avg_flux'],
        exposure_data['std_x_error_pixels'],
        alpha=0.5,
        s=20,
        color='blue',
        edgecolors='black',
        linewidth=0.5
    )

    ax.set_title(f'Exposure: {exp_time} ms')
    ax.set_xlabel('Average Flux (DN)')
    ax.set_ylabel('std_x_error_pixels')
    ax.set_xscale('log')
    ax.set_xlim(flux_min_global, flux_max_global)
    ax.set_ylim(-0.25, 0.25)
    ax.grid(True, alpha=0.3)

plt.tight_layout(rect=[0, 0, 1, 0.96])
fig4.suptitle(f'Tracking Error vs Flux - {sensor_name}', fontsize=16, fontweight='bold', y=0.98)
error_flux_output = output_dir / 'x_error_vs_flux.png'
plt.savefig(error_flux_output, dpi=150, bbox_inches='tight')
print(f"Saved error vs flux plot to: {error_flux_output}")

if not args.no_show:
    plt.show()

print(f"\nAll plots saved to: {output_dir}")
