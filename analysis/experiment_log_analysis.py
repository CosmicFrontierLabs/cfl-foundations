#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path
import argparse

def load_experiment_data(csv_path):
    """Load experiment log CSV with proper header handling"""
    with open(csv_path, 'r') as f:
        lines = f.readlines()
    
    # Find where actual data starts (after "Experiment Data:" line)
    data_start_idx = None
    header_idx = None
    for i, line in enumerate(lines):
        if line.strip() == "Experiment Data:":
            header_idx = i + 1
            data_start_idx = i + 2
            break
    
    if data_start_idx is None:
        raise ValueError("Could not find 'Experiment Data:' marker in CSV")
    
    # Parse header
    header = [col.strip() for col in lines[header_idx].strip().split(',')]
    
    # Parse data rows manually to handle variable number of magnitude values
    data_rows = []
    for line in lines[data_start_idx:]:
        line = line.strip()
        if not line:
            continue
            
        # Split by comma but be careful with the variable magnitude values
        parts = line.split(',')
        
        # First 6 columns are fixed
        row_data = {
            'experiment_num': int(parts[0]),
            'sensor_name': parts[1].strip(),
            'ra_degrees': float(parts[2]),
            'dec_degrees': float(parts[3]),
            'detected_count': int(parts[4]),
            'alignment_error_pix': float(parts[5])
        }
        
        # Remaining values are detected magnitudes
        if len(parts) > 6:
            magnitudes = [float(m.strip()) for m in parts[6:] if m.strip()]
            row_data['detected_magnitudes'] = magnitudes
            row_data['n_magnitudes'] = len(magnitudes)
        else:
            row_data['detected_magnitudes'] = []
            row_data['n_magnitudes'] = 0
            
        data_rows.append(row_data)
    
    df = pd.DataFrame(data_rows)
    return df

def plot_pixel_precision_cdf(df, output_path, max_threshold=1.0, show=False):
    """Create cumulative distribution plot of pixel precision for each sensor"""
    fig, ax = plt.subplots(1, 1, figsize=(12, 8))
    
    # Get unique sensors
    sensors = df['sensor_name'].unique()
    colors = plt.cm.viridis(np.linspace(0, 1, len(sensors)))
    
    # For each sensor, calculate cumulative distribution
    for sensor, color in zip(sensors, colors):
        sensor_data = df[df['sensor_name'] == sensor]
        
        # Get alignment errors (pixel precision)
        errors = sensor_data['alignment_error_pix'].values
        
        # Create range of thresholds from 0 to max_threshold
        thresholds = np.logspace(-3, np.log10(max_threshold), 1000)
        
        # Calculate percentage below each threshold
        percentages = []
        for threshold in thresholds:
            below_threshold = np.sum(errors <= threshold)
            percentage = (below_threshold / len(errors)) * 100
            percentages.append(percentage)
        
        # Plot line for this sensor
        ax.plot(thresholds, percentages, label=sensor, color=color, linewidth=2)
    
    # Set x-axis to log scale
    ax.set_xscale('log')
    
    # Labels and title
    ax.set_xlabel('Pixel Precision Threshold (pixels)', fontsize=12)
    ax.set_ylabel('Percentage of Experiments Below Threshold (%)', fontsize=12)
    ax.set_title('Cumulative Distribution of Alignment Precision by Sensor', fontsize=14, pad=20)
    
    # Grid
    ax.grid(True, alpha=0.3, linestyle='--', which='both')
    
    # Legend
    ax.legend(loc='lower right', fontsize=10)
    
    # Set y-axis limits
    ax.set_ylim(0, 100)
    
    # Add some reference lines
    ax.axhline(y=50, color='gray', linestyle=':', alpha=0.5, label='50%')
    ax.axhline(y=90, color='gray', linestyle=':', alpha=0.5, label='90%')
    ax.axvline(x=0.1, color='gray', linestyle=':', alpha=0.5, label='0.1 pixel')
    
    plt.tight_layout()
    plt.savefig(output_path, dpi=300)
    print(f"Saved pixel precision CDF plot to {output_path}")
    if show:
        plt.show()
    else:
        plt.close()

def plot_ra_dec_coverage(df, output_path, show=False):
    """Create scatter plot of RA/Dec positions for all experiments"""
    fig, ax = plt.subplots(1, 1, figsize=(12, 8))
    
    # Get unique experiments (each experiment tests all sensors at same position)
    unique_experiments = df.drop_duplicates(subset=['experiment_num'])[['ra_degrees', 'dec_degrees']]
    
    # Scatter plot
    scatter = ax.scatter(unique_experiments['ra_degrees'], 
                        unique_experiments['dec_degrees'],
                        alpha=0.6, 
                        s=30,
                        c=range(len(unique_experiments)),
                        cmap='viridis',
                        edgecolors='black',
                        linewidth=0.5)
    
    # Add colorbar to show experiment progression
    cbar = plt.colorbar(scatter, ax=ax)
    cbar.set_label('Experiment Number', rotation=270, labelpad=20)
    
    # Labels and title
    ax.set_xlabel('Right Ascension (degrees)', fontsize=12)
    ax.set_ylabel('Declination (degrees)', fontsize=12)
    ax.set_title('Sky Coverage of Sensor Shootout Experiments', fontsize=14, pad=20)
    
    # Add grid
    ax.grid(True, alpha=0.3, linestyle='--')
    
    # Set proper RA limits (0-360 degrees)
    ax.set_xlim(0, 360)
    ax.set_ylim(-90, 90)
    
    # Add some statistics
    n_experiments = len(unique_experiments)
    ax.text(0.02, 0.98, f'Total Experiments: {n_experiments}', 
            transform=ax.transAxes, 
            verticalalignment='top',
            bbox=dict(boxstyle='round', facecolor='white', alpha=0.8))
    
    plt.tight_layout()
    plt.savefig(output_path, dpi=300)
    print(f"Saved RA/Dec coverage plot to {output_path}")
    if show:
        plt.show()
    else:
        plt.close()

def parse_args():
    parser = argparse.ArgumentParser(description='Analyze sensor shootout experiment data')
    parser.add_argument(
        '--output', '-o',
        type=str,
        default='plots/experiment_log',
        help='Output path for plots (folder/prefix format). Default: plots/experiment_log'
    )
    parser.add_argument(
        '--input', '-i',
        type=str,
        default='experiment_log.csv',
        help='Input CSV file path. Default: experiment_log.csv'
    )
    parser.add_argument(
        '--show', '-s',
        action='store_true',
        help='Show plots interactively (default: False)'
    )
    parser.add_argument(
        '--max-threshold', '-t',
        type=float,
        default=1.0,
        help='Maximum pixel precision threshold for cumulative distribution plot (default: 1.0 pixels)'
    )
    return parser.parse_args()

def main():
    args = parse_args()
    
    # Parse output path
    output_path = Path(args.output)
    if '/' in args.output:
        output_dir = output_path.parent
        prefix = output_path.name
    else:
        output_dir = Path('.')
        prefix = args.output
    
    # Create output directory if needed
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Load data
    csv_path = Path(args.input)
    if not csv_path.is_absolute():
        csv_path = Path(__file__).parent.parent / csv_path
    
    print(f"Loading data from {csv_path}")
    df = load_experiment_data(csv_path)
    print(f"Loaded {len(df)} rows of data")
    print(f"Unique experiments: {df['experiment_num'].nunique()}")
    print(f"Sensors: {df['sensor_name'].unique()}")
    
    # Create RA/Dec plot
    ra_dec_path = output_dir / f"{prefix}_ra_dec_coverage.png"
    plot_ra_dec_coverage(df, ra_dec_path, show=args.show)
    
    # Create pixel precision CDF plot
    precision_path = output_dir / f"{prefix}_pixel_precision_cdf.png"
    plot_pixel_precision_cdf(df, precision_path, max_threshold=args.max_threshold, show=args.show)

if __name__ == "__main__":
    main()