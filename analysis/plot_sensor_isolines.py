#!/usr/bin/env python3
"""
Sensor Floor Detection Rate Isoline Plotter

Creates 2D isoline plots showing 95% detection rate contours for each sensor.
- X-axis: Star magnitude 
- Y-axis: PSF sampling (Q_Value)
- Isolines: One per exposure time at 95% detection rate level

Usage: python plot_sensor_isolines.py sensor_floor_results.csv
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from scipy.interpolate import griddata
from scipy.ndimage import median_filter
import sys
import os
import argparse
from pathlib import Path

def parse_sensor_data(csv_file, precision_threshold=None):
    """Parse the sensor floor CSV file and extract all 3 data matrices for each sensor."""
    
    sensors_data = {}
    current_sensor = None
    current_block = None
    
    with open(csv_file, 'r') as f:
        lines = f.readlines()
    
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        
        # Look for sensor sections
        if line.startswith("SENSOR: "):
            current_sensor = line.replace("SENSOR: ", "")
            sensors_data[current_sensor] = {
                'detection_rates': [],
                'mean_errors': [],
                'rms_errors': [],
                'magnitudes': [],
                'q_values': [],
                'exposures': []
            }
            current_block = None
            i += 1
            continue
        
        # Identify data block types
        if line == "Detection Rate Matrix (%)":
            current_block = 'detection_rates'
            i += 1
            # Parse header for magnitudes
            if i < len(lines):
                header_line = lines[i].strip()
                if header_line.startswith("Q_Value,Exposure_ms,"):
                    mag_values = header_line.split(',')[2:]
                    mag_values = [float(mag) for mag in mag_values if mag.strip()]
                    sensors_data[current_sensor]['magnitudes'] = mag_values
                i += 1
            continue
        elif line == "Mean Position Error Matrix (milliarcseconds)":
            current_block = 'mean_errors'
            i += 2  # Skip header line
            continue
        elif line == "RMS Position Error Matrix (pixels)":
            current_block = 'rms_errors'
            i += 2  # Skip header line
            continue
        
        # Parse data rows for current block
        if current_block and current_sensor and ',' in line and not line.startswith("---"):
            parts = line.split(',')
            if len(parts) >= 3:
                try:
                    q_value = float(parts[0])
                    exposure = int(parts[1])
                    
                    # Extract data values using header length as reference
                    expected_length = len(sensors_data[current_sensor]['magnitudes'])
                    data_values = []
                    
                    # Process each expected column
                    for j in range(expected_length):
                        if j + 2 < len(parts) and parts[j + 2].strip():
                            data_values.append(float(parts[j + 2]))
                        else:
                            data_values.append(np.nan)  # Missing or empty value
                    
                    sensors_data[current_sensor][current_block].append(data_values)
                    # Only store q_values and exposures once (from detection rates)
                    if current_block == 'detection_rates':
                        sensors_data[current_sensor]['q_values'].append(q_value)
                        sensors_data[current_sensor]['exposures'].append(exposure)
                
                except (ValueError, IndexError):
                    pass
        
        # Reset block when hitting section boundaries
        if line.startswith("SENSOR:") or line.startswith("---"):
            current_block = None
        
        i += 1
    
    # Convert to numpy arrays and apply masking
    for sensor in sensors_data:
        data = sensors_data[sensor]
        if data['detection_rates']:
            # Convert arrays directly - they should all be same shape
            data['detection_rates'] = np.array(data['detection_rates'])
            data['mean_errors'] = np.array(data['mean_errors']) if data['mean_errors'] else np.array([])
            data['rms_errors'] = np.array(data['rms_errors']) if data['rms_errors'] else np.array([])
            
            data['q_values'] = np.array(data['q_values'])
            data['exposures'] = np.array(data['exposures'])
            data['magnitudes'] = np.array(data['magnitudes'])
            
            # Apply precision masking if threshold is set
            if precision_threshold is not None and data['rms_errors'].size > 0:
                # Mask detection rates where RMS error > threshold
                # Mask detection rates where RMS error > threshold
                mask = data['rms_errors'] > precision_threshold
                masked_detection_rates = data['detection_rates'].copy()
                masked_detection_rates[mask] = 0.0  # Set to 0% detection rate
                data['detection_rates'] = masked_detection_rates
    
    return sensors_data

def create_isoline_plot(sensor_name, sensor_data, output_dir="plots", enable_smoothing=False, output_suffix="", contour_level=0.95):
    """Create 2D isoline plot for a single sensor."""
    
    detection_rates = sensor_data['detection_rates']  # Shape: (n_measurements, n_magnitudes)
    q_values = sensor_data['q_values']
    exposures = sensor_data['exposures']
    magnitudes = sensor_data['magnitudes']
    
    if len(detection_rates) == 0:
        print(f"No data for sensor {sensor_name}")
        return
    
    # Get unique exposure times
    unique_exposures = np.unique(exposures)
    
    # Create figure
    plt.figure(figsize=(12, 8))
    
    # Define colors for different exposure times
    colors = plt.cm.viridis(np.linspace(0, 1, len(unique_exposures)))
    
    # For each exposure time, find the detection rate isoline
    for exp_idx, exposure in enumerate(unique_exposures):
        # Get data for this exposure
        exp_mask = exposures == exposure
        exp_q_values = q_values[exp_mask]
        exp_detection_rates = detection_rates[exp_mask] / 100.0  # Convert percentage to fraction
        
        if len(exp_q_values) == 0:
            continue
        
        # Create meshgrid for interpolation
        mag_min, mag_max = magnitudes.min(), magnitudes.max()
        q_min, q_max = exp_q_values.min(), exp_q_values.max()
        
        # Create high-resolution grid with magnitude-based scaling
        # Use flux-based spacing (10^(-0.4*mag)) for natural stellar brightness scaling
        mag_grid = np.linspace(mag_min, mag_max, 200)
        flux_grid = 10**(-0.4 * mag_grid)  # Convert to flux for spacing
        q_grid = np.linspace(q_min, q_max, 200)
        Flux_grid, Q_grid = np.meshgrid(flux_grid, q_grid)
        
        # Prepare data points for interpolation using flux coordinates
        points = []
        values = []
        
        for i, q_val in enumerate(exp_q_values):
            for j, mag_val in enumerate(magnitudes):
                if j < len(exp_detection_rates[i]):
                    detection_rate = exp_detection_rates[i][j]
                    if not np.isnan(detection_rate):
                        flux_val = 10**(-0.4 * mag_val)  # Convert magnitude to flux
                        points.append([flux_val, q_val])
                        values.append(detection_rate)
        
        if len(points) < 4:  # Need at least 4 points for interpolation
            print(f"Not enough data points for exposure {exposure}ms")
            continue
        
        points = np.array(points)
        values = np.array(values)
        
        # Interpolate onto flux-based grid
        try:
            interpolated = griddata(points, values, (Flux_grid, Q_grid), method='cubic', fill_value=0)
            
            # Apply median filtering if enabled (preserves edges better than gaussian)
            if enable_smoothing:
                interpolated = median_filter(interpolated, size=3)
            
            # Create contour using flux coordinates but convert back to magnitude for display
            Mag_grid_display = -2.5 * np.log10(Flux_grid)  # Convert flux back to magnitude for display
            contours = plt.contour(Mag_grid_display, Q_grid, interpolated, levels=[contour_level], 
                                 colors=[colors[exp_idx]], linewidths=2)
            
            # Label the contour
            if len(contours.collections) > 0 and len(contours.collections[0].get_paths()) > 0:
                plt.clabel(contours, fmt=f'{exposure}ms', fontsize=10, inline=True)
        
        except Exception as e:
            print(f"Failed to create contour for exposure {exposure}ms: {e}")
            continue
    
    # Customize plot
    plt.xlabel('Star Magnitude', fontsize=12)
    plt.ylabel('PSF Sampling (Q_Value)', fontsize=12)
    plt.title(f'{sensor_name} - {contour_level*100:.0f}% Detection Rate Isolines', fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3)
    
    # Use normal linear scale for x-axis
    # plt.xscale('log')  # Removed log scaling
    
    # Don't invert x-axis - use normal convention (brighter stars = higher values on right)
    # plt.gca().invert_xaxis()
    
    # Create custom legend for exposure times
    legend_elements = []
    for exp_idx, exposure in enumerate(unique_exposures):
        legend_elements.append(plt.Line2D([0], [0], color=colors[exp_idx], lw=2, 
                                        label=f'{exposure}ms'))
    
    # Add faint dotted horizontal line at (1.029*2)/1.22 - Nyquist sampling limit
    nyquist_y = (1.029*2)/1.22
    plt.axhline(y=nyquist_y, color='gray', linestyle=':', alpha=0.4)
    
    # Add text label on the right side
    plt.text(plt.xlim()[1], nyquist_y, ' Nyquist sampling', 
             verticalalignment='center', fontsize=9, alpha=0.6)
    
    plt.legend(handles=legend_elements, title='Exposure Time', loc='upper right')
    
    # Save plot
    os.makedirs(output_dir, exist_ok=True)
    safe_sensor_name = sensor_name.replace('/', '_').replace(' ', '_')
    filename = f'{safe_sensor_name}_isolines{output_suffix}.png'
    output_file = os.path.join(output_dir, filename)
    plt.savefig(output_file, dpi=300, bbox_inches='tight')
    print(f"Saved plot: {output_file}")
    
    plt.close()

def main():
    parser = argparse.ArgumentParser(description='Create sensor detection rate isoline plots')
    parser.add_argument('csv_file', help='Path to sensor_floor_results.csv file')
    parser.add_argument('--smooth', action='store_true', default=False,
                        help='Enable median filtering of interpolated data (default: off)')
    parser.add_argument('--precision-threshold', type=float, default=None,
                        help='Mask detection rates where RMS position error > threshold pixels (default: no masking)')
    parser.add_argument('--output-suffix', type=str, default='',
                        help='Suffix to add to output plot filenames (default: none)')
    parser.add_argument('--contour-level', type=float, default=0.95,
                        help='Detection rate contour level (0.0-1.0, default: 0.95 for 95%%)')
    
    args = parser.parse_args()
    csv_file = args.csv_file
    
    if not os.path.exists(csv_file):
        print(f"Error: File {csv_file} not found")
        sys.exit(1)
    
    print(f"Parsing sensor data from {csv_file}...")
    if args.precision_threshold is not None:
        print(f"Masking areas with RMS error > {args.precision_threshold} pixels")
    sensors_data = parse_sensor_data(csv_file, args.precision_threshold)
    
    if not sensors_data:
        print("No sensor data found in CSV file")
        sys.exit(1)
    
    print(f"Found {len(sensors_data)} sensors:")
    for sensor in sensors_data.keys():
        print(f"  - {sensor}")
    
    smoothing_text = ' with median filtering' if args.smooth else ''
    masking_text = f' (precision masked at {args.precision_threshold}px)' if args.precision_threshold else ''
    print(f"\nGenerating isoline plots{smoothing_text}{masking_text}...")
    for sensor_name, sensor_data in sensors_data.items():
        print(f"Processing {sensor_name}...")
        create_isoline_plot(sensor_name, sensor_data, enable_smoothing=args.smooth, 
                          output_suffix=args.output_suffix, contour_level=args.contour_level)
    
    print("\nAll plots generated successfully!")

if __name__ == "__main__":
    main()