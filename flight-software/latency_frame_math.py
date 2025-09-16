#!/usr/bin/env python3
import numpy as np
from PIL import Image
import matplotlib.pyplot as plt
import sys

def analyze_frame(image_path):
    # Load the PNG image
    img = Image.open(image_path)
    img_array = np.array(img)
    
    print(f"Image shape: {img_array.shape}")
    print(f"Data type: {img_array.dtype}")
    print(f"Min value: {img_array.min()}, Max value: {img_array.max()}")
    
    # Calculate row medians (median across columns for each row)
    row_medians = np.median(img_array, axis=1)
    
    # Create the plot
    plt.figure(figsize=(12, 6))
    
    # Plot row medians
    plt.plot(row_medians, linewidth=0.5)
    plt.xlabel('Row Index')
    plt.ylabel('Median Pixel Value')
    plt.title(f'Row Medians - {image_path}')
    plt.grid(True, alpha=0.3)
    
    plt.tight_layout()
    
    # Save the plot
    output_name = image_path.replace('.png', '_row_medians.png')
    plt.savefig(output_name, dpi=150)
    print(f"Plot saved as: {output_name}")
    
    # Also show statistics
    print(f"\nRow medians statistics:")
    print(f"  Mean of row medians: {row_medians.mean():.2f}")
    print(f"  Std of row medians: {row_medians.std():.2f}")
    print(f"  Min row median: {row_medians.min():.2f}")
    print(f"  Max row median: {row_medians.max():.2f}")
    print(f"  Median of row medians: {np.median(row_medians):.2f}")
    
    plt.show()

if __name__ == "__main__":
    if len(sys.argv) > 1:
        analyze_frame(sys.argv[1])
    else:
        # Default to the full capture
        analyze_frame("full_capture.png")