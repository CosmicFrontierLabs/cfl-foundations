#!/usr/bin/env python3
"""Plot gyro data from parsed CSV."""

import argparse
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt


def load_full_csv(csv_path: str) -> pd.DataFrame:
    """Load full CSV including skipped rows."""
    return pd.read_csv(csv_path, low_memory=False)


def load_data(csv_path: str) -> pd.DataFrame:
    """Load CSV and filter to data rows only."""
    df = pd.read_csv(csv_path, low_memory=False)
    return df[df["type"] == "data"].copy()


def compute_timestamp_diffs(timestamps: np.ndarray, max_val: int = 2**32) -> np.ndarray:
    """Compute timestamp differences, correcting for rollover."""
    diffs = np.diff(timestamps.astype(np.int64))
    # Correct for rollover - if diff is negative, add max_val
    diffs = np.where(diffs < 0, diffs + max_val, diffs)
    return diffs


def find_skip_indices(df_full: pd.DataFrame) -> list[int]:
    """Find sample indices where skipped data occurred."""
    is_data = (df_full["type"] == "data").values
    # Cumsum of data rows gives us the data index at each position
    data_idx_at_pos = np.cumsum(is_data)
    # Where we have skipped rows, get the data index (which is the index of next data row)
    skip_mask = ~is_data
    return data_idx_at_pos[skip_mask].tolist()


def plot_timestamp_sawtooth(df: pd.DataFrame, skip_indices: list[int], output_path: str = None):
    """Plot timestamps showing sawtooth pattern with markers for missing data."""
    timestamps = df["gyro_time"].values
    sample_idx = np.arange(len(timestamps))

    fig, ax = plt.subplots(figsize=(14, 6))

    ax.plot(sample_idx, timestamps, linewidth=0.5, alpha=0.8)

    # Add vertical lines where data was skipped
    for idx in skip_indices:
        ax.axvline(idx, color="red", linestyle="-", alpha=0.2, linewidth=1)

    ax.set_xlabel("Sample Index")
    ax.set_ylabel("Timestamp (counts)")
    ax.set_title(f"Gyro Timestamps (sawtooth from rollover)\nRed lines = missing data ({len(skip_indices)} gaps)")

    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150)
        print(f"Saved to {output_path}")


def plot_timestamp_histogram(df: pd.DataFrame, output_path: str = None):
    """Plot histogram of timestamp differences."""
    timestamps = df["gyro_time"].values
    diffs = compute_timestamp_diffs(timestamps)

    fig, ax = plt.subplots(figsize=(12, 6))

    # Use log scale for y-axis since most values cluster tightly
    ax.hist(diffs, bins=100, edgecolor="black", alpha=0.7)
    ax.set_yscale("log")

    ax.set_xlabel("Timestamp Difference (counts)")
    ax.set_ylabel("Frequency (log scale)")
    ax.set_title(f"Gyro Timestamp Differences\n(n={len(diffs):,} samples)")

    # Add stats
    median = np.median(diffs)
    mean = np.mean(diffs)
    std = np.std(diffs)
    ax.axvline(median, color="red", linestyle="--", label=f"Median: {median:.1f}")
    ax.axvline(mean, color="green", linestyle="--", label=f"Mean: {mean:.1f}")
    ax.legend()

    # Add text with stats
    stats_text = f"Std: {std:.1f}\nMin: {diffs.min()}\nMax: {diffs.max()}"
    ax.text(0.98, 0.98, stats_text, transform=ax.transAxes,
            verticalalignment="top", horizontalalignment="right",
            bbox=dict(boxstyle="round", facecolor="wheat", alpha=0.5))

    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150)
        print(f"Saved to {output_path}")


def plot_timestamp_diffs_timeseries(df: pd.DataFrame, skip_indices: list[int], output_path: str = None):
    """Plot timestamp differences over time (handling u16 rollover in lower bits)."""
    timestamps = df["gyro_time"].values

    # Extract lower 16 bits (the part that actually increments)
    lower_bits = (timestamps % 65536).astype(np.int32)

    # Compute differences with u16 rollover correction
    diffs = np.diff(lower_bits)
    diffs = np.where(diffs < 0, diffs + 65536, diffs)

    sample_idx = np.arange(len(diffs))

    fig, ax = plt.subplots(figsize=(14, 6))

    ax.plot(sample_idx, diffs, linewidth=0.3, alpha=0.8)

    # Add vertical lines where data was skipped
    for idx in skip_indices:
        ax.axvline(idx, color="red", linestyle="-", alpha=0.2, linewidth=1)

    ax.set_xlabel("Sample Index")
    ax.set_ylabel("Timestamp Difference (counts)")
    ax.set_title(f"Timestamp Differences Over Time (u16 rollover corrected)\nRed lines = missing data ({len(skip_indices)} gaps)\n(n={len(diffs):,} samples)")
    ax.grid(True, alpha=0.3)

    # Add stats
    median = np.median(diffs)
    mean = np.mean(diffs)
    std = np.std(diffs)

    # Add text box with detailed stats
    stats_text = f"Mean: {mean:.2f}\nMedian: {median:.2f}\nStd: {std:.2f}\nMin: {diffs.min()}\nMax: {diffs.max()}"
    ax.text(0.02, 0.98, stats_text, transform=ax.transAxes,
            verticalalignment="top", horizontalalignment="left",
            fontsize=9, bbox=dict(boxstyle="round", facecolor="wheat", alpha=0.5))

    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150)
        print(f"Saved to {output_path}")


def plot_angles(df: pd.DataFrame, skip_indices: list[int], output_path: str = None):
    """Plot raw and filtered angle data over time (in arcseconds)."""
    sample_idx = np.arange(len(df))

    fig, axes = plt.subplots(3, 2, figsize=(16, 12), sharex=True)

    # Angle fields: (raw_field, filtered_field, axis_label)
    angle_fields = [
        ("raw_ang_x_arcsec", "fil_ang_x_arcsec", "X-axis"),
        ("raw_ang_y_arcsec", "fil_ang_y_arcsec", "Y-axis"),
        ("raw_ang_z_arcsec", "fil_ang_z_arcsec", "Z-axis"),
    ]

    for idx, (raw_field, filt_field, axis_label) in enumerate(angle_fields):
        # Raw angles (left column)
        ax_raw = axes[idx, 0]
        raw_values = pd.to_numeric(df[raw_field], errors='coerce').values
        valid_mask = ~np.isnan(raw_values)

        if valid_mask.any():
            ax_raw.plot(sample_idx[valid_mask], raw_values[valid_mask], linewidth=0.3, alpha=0.8, color='blue')

            # Add vertical lines for skip events
            for skip_idx in skip_indices:
                ax_raw.axvline(skip_idx, color="red", linestyle="-", alpha=0.2, linewidth=1)

            ax_raw.set_ylabel(f"{axis_label} Raw\n(arcsec)")
            ax_raw.grid(True, alpha=0.3)

            # Stats
            mean = np.mean(raw_values[valid_mask])
            std = np.std(raw_values[valid_mask])
            ax_raw.text(0.02, 0.98, f"Mean: {mean:.2f}\nStd: {std:.2f}",
                       transform=ax_raw.transAxes, verticalalignment="top",
                       fontsize=9, bbox=dict(boxstyle="round", facecolor="wheat", alpha=0.5))

        # Filtered angles (right column)
        ax_filt = axes[idx, 1]
        filt_values = pd.to_numeric(df[filt_field], errors='coerce').values
        valid_mask = ~np.isnan(filt_values)

        if valid_mask.any():
            ax_filt.plot(sample_idx[valid_mask], filt_values[valid_mask], linewidth=0.3, alpha=0.8, color='green')

            # Add vertical lines for skip events
            for skip_idx in skip_indices:
                ax_filt.axvline(skip_idx, color="red", linestyle="-", alpha=0.2, linewidth=1)

            ax_filt.set_ylabel(f"{axis_label} Filtered\n(arcsec)")
            ax_filt.grid(True, alpha=0.3)

            # Stats
            mean = np.mean(filt_values[valid_mask])
            std = np.std(filt_values[valid_mask])
            ax_filt.text(0.02, 0.98, f"Mean: {mean:.2f}\nStd: {std:.2f}",
                        transform=ax_filt.transAxes, verticalalignment="top",
                        fontsize=9, bbox=dict(boxstyle="round", facecolor="wheat", alpha=0.5))

    axes[-1, 0].set_xlabel("Sample Index")
    axes[-1, 1].set_xlabel("Sample Index")
    axes[0, 0].set_title(f"Raw Angle Measurements\n(n={len(df):,} samples)")
    axes[0, 1].set_title(f"Filtered Angle Measurements\n(n={len(df):,} samples)")

    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150)
        print(f"Saved to {output_path}")


def plot_temperatures(df: pd.DataFrame, output_path: str = None):
    """Plot all temperature channels over time (decoded to Celsius)."""
    sample_idx = np.arange(len(df))

    fig, axes = plt.subplots(4, 1, figsize=(14, 10), sharex=True)

    # Updated field names to use decoded Celsius columns
    temp_fields = [
        ("board_temp_c", "Board Temperature"),
        ("sia_fil_temp_c", "SIA Filter Temperature"),
        ("org_fil_temp_c", "Organizer Temperature"),
        ("inter_temp_c", "Interface Temperature"),
    ]

    for ax, (field, label) in zip(axes, temp_fields):
        # Handle potential NaN values from failed conversions or missing sensors
        values = pd.to_numeric(df[field], errors='coerce').values

        # Filter out NaN/empty values for plotting
        valid_mask = ~np.isnan(values)
        valid_indices = sample_idx[valid_mask]
        valid_values = values[valid_mask]

        if len(valid_values) > 0:
            ax.plot(valid_indices, valid_values, linewidth=0.3, alpha=0.8)
            ax.set_ylabel(f"{label}\n(°C)")
            ax.grid(True, alpha=0.3)

            # Add stats (only for valid values)
            mean = np.mean(valid_values)
            std = np.std(valid_values)
            min_val = np.min(valid_values)
            max_val = np.max(valid_values)
            ax.text(0.02, 0.95, f"Mean: {mean:.2f}°C, Std: {std:.2f}°C\nMin: {min_val:.2f}°C, Max: {max_val:.2f}°C",
                    transform=ax.transAxes, verticalalignment="top",
                    fontsize=9, bbox=dict(boxstyle="round", facecolor="wheat", alpha=0.5))
        else:
            # No valid data for this sensor (e.g., Raw/Filtered messages don't have all sensors)
            ax.text(0.5, 0.5, f"No data for {label}",
                    transform=ax.transAxes, ha='center', va='center',
                    fontsize=12, color='gray')
            ax.set_ylabel(f"{label}\n(°C)")
            ax.grid(True, alpha=0.3)

    axes[-1].set_xlabel("Sample Index")
    axes[0].set_title(f"Gyro Temperature Channels (Decoded)\n(n={len(df):,} samples)")

    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150)
        print(f"Saved to {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Plot gyro data from parsed CSV")
    parser.add_argument("csv_file", help="Input CSV file from parse_dump")
    parser.add_argument("-o", "--output", help="Output PNG file")
    parser.add_argument("--no-show", action="store_true", help="Don't display the plot")
    args = parser.parse_args()

    print(f"Loading {args.csv_file}...")
    df_full = load_full_csv(args.csv_file)
    df = df_full[df_full["type"] == "data"].copy()
    print(f"Loaded {len(df):,} data records")

    skip_indices = find_skip_indices(df_full)
    print(f"Found {len(skip_indices)} skip events")

    plot_timestamp_sawtooth(df, skip_indices, args.output)
    plot_timestamp_diffs_timeseries(df, skip_indices)
    plot_angles(df, skip_indices)
    plot_temperatures(df)

    if not args.no_show:
        plt.show()


if __name__ == "__main__":
    main()
