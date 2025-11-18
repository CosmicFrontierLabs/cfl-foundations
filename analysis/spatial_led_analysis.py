#!/usr/bin/env python3
"""
Analyze spatial distribution of LED signal in alternating frames.
Find rolling shutter transition row.
"""

import numpy as np
from astropy.io import fits
from PIL import Image
import matplotlib.pyplot as plt
import sys
from pathlib import Path

def analyze_spatial_distribution(image_files):
    """Analyze spatial distribution of signal across alternating frames."""

    frames = []
    for f in image_files:
        if f.suffix.lower() in ['.fits', '.fit']:
            with fits.open(f) as hdul:
                data = hdul[1].data if len(hdul) > 1 and hdul[1].data is not None else hdul[0].data
                frames.append(data.astype(float))
        else:
            img = Image.open(f)
            frames.append(np.array(img).astype(float))

    if len(frames) < 2:
        print(f"Need at least 2 frames, got {len(frames)}")
        return

    print(f"Loaded {len(frames)} frames")
    print(f"Frame shape: {frames[0].shape}")

    led_on_frames = frames[0::2]
    led_off_frames = frames[1::2]

    print(f"LED ON frames: {len(led_on_frames)}")
    print(f"LED OFF frames: {len(led_off_frames)}")

    min_count = min(len(led_on_frames), len(led_off_frames))

    if min_count == 0:
        print("Need at least one ON and one OFF frame")
        return

    led_on_avg = np.mean(led_on_frames[:min_count], axis=0)
    led_off_avg = np.mean(led_off_frames[:min_count], axis=0)

    diff = led_on_avg - led_off_avg

    print(f"\nLED ON average: min={led_on_avg.min():.1f}, max={led_on_avg.max():.1f}, mean={led_on_avg.mean():.1f}")
    print(f"LED OFF average: min={led_off_avg.min():.1f}, max={led_off_avg.max():.1f}, mean={led_off_avg.mean():.1f}")
    print(f"Difference: min={diff.min():.1f}, max={diff.max():.1f}, mean={diff.mean():.1f}, std={diff.std():.1f}")

    print(f"\nRow-wise statistics:")
    row_means_on = led_on_avg.mean(axis=1)
    row_means_off = led_off_avg.mean(axis=1)
    row_diff = row_means_on - row_means_off

    print(f"Row mean difference: min={row_diff.min():.1f}, max={row_diff.max():.1f}")
    print(f"Rows with largest positive diff: {np.argsort(row_diff)[-5:]}")
    print(f"Rows with largest negative diff: {np.argsort(row_diff)[:5]}")

    print(f"\nColumn-wise statistics:")
    col_means_on = led_on_avg.mean(axis=0)
    col_means_off = led_off_avg.mean(axis=0)
    col_diff = col_means_on - col_means_off

    print(f"Column mean difference: min={col_diff.min():.1f}, max={col_diff.max():.1f}")

    gradient = np.gradient(row_diff)
    transition_row = np.argmax(np.abs(gradient))
    print(f"\nTransition row: {transition_row} (gradient = {gradient[transition_row]:.1f})")

    fig, axes = plt.subplots(2, 3, figsize=(18, 12))

    vmin = min(led_on_avg.min(), led_off_avg.min())
    vmax = max(led_on_avg.max(), led_off_avg.max())

    im0 = axes[0, 0].imshow(led_on_avg, cmap='gray', vmin=vmin, vmax=vmax, origin='lower', aspect='auto')
    axes[0, 0].axhline(y=transition_row, color='red', linestyle='--', linewidth=2)
    axes[0, 0].set_title(f'LED ON ({len(led_on_frames[:min_count])} frames)')
    axes[0, 0].set_xlabel('Column')
    axes[0, 0].set_ylabel('Row')
    plt.colorbar(im0, ax=axes[0, 0])

    im1 = axes[0, 1].imshow(led_off_avg, cmap='gray', vmin=vmin, vmax=vmax, origin='lower', aspect='auto')
    axes[0, 1].axhline(y=transition_row, color='red', linestyle='--', linewidth=2)
    axes[0, 1].set_title(f'LED OFF ({len(led_off_frames[:min_count])} frames)')
    axes[0, 1].set_xlabel('Column')
    axes[0, 1].set_ylabel('Row')
    plt.colorbar(im1, ax=axes[0, 1])

    diff_absmax = max(abs(diff.min()), abs(diff.max()))
    im2 = axes[0, 2].imshow(diff, cmap='RdBu_r', vmin=-diff_absmax, vmax=diff_absmax, origin='lower', aspect='auto')
    axes[0, 2].axhline(y=transition_row, color='yellow', linestyle='--', linewidth=2)
    axes[0, 2].set_title('Difference (ON - OFF)')
    axes[0, 2].set_xlabel('Column')
    axes[0, 2].set_ylabel('Row')
    plt.colorbar(im2, ax=axes[0, 2])

    axes[1, 0].plot(row_means_on, label='LED ON', alpha=0.7, linewidth=2)
    axes[1, 0].plot(row_means_off, label='LED OFF', alpha=0.7, linewidth=2)
    axes[1, 0].axvline(x=transition_row, color='red', linestyle='--', linewidth=2, alpha=0.5)
    axes[1, 0].set_xlabel('Row')
    axes[1, 0].set_ylabel('Mean pixel value')
    axes[1, 0].set_title('Row-wise means')
    axes[1, 0].legend()
    axes[1, 0].grid(True, alpha=0.3)

    axes[1, 1].plot(row_diff, linewidth=2)
    axes[1, 1].axhline(y=0, color='gray', linestyle='--', alpha=0.5)
    axes[1, 1].axvline(x=transition_row, color='red', linestyle='--', linewidth=2, alpha=0.5)
    axes[1, 1].set_xlabel('Row')
    axes[1, 1].set_ylabel('Mean difference (ON - OFF)')
    axes[1, 1].set_title('Row-wise difference')
    axes[1, 1].grid(True, alpha=0.3)

    axes[1, 2].plot(gradient, linewidth=2)
    axes[1, 2].axhline(y=0, color='gray', linestyle='--', alpha=0.5)
    axes[1, 2].axvline(x=transition_row, color='red', linestyle='--', linewidth=2, alpha=0.5)
    axes[1, 2].set_xlabel('Row')
    axes[1, 2].set_ylabel('Gradient')
    axes[1, 2].set_title(f'Gradient (transition at row {transition_row})')
    axes[1, 2].grid(True, alpha=0.3)

    plt.tight_layout()
    output_file = 'spatial_led_analysis.png'
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    print(f"\nSaved analysis to {output_file}")
    plt.close()

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: python3 spatial_led_analysis.py <frame0.png|fits> <frame1.png|fits> ...")
        sys.exit(1)

    image_files = [Path(f) for f in sys.argv[1:]]

    for f in image_files:
        if not f.exists():
            print(f"File not found: {f}")
            sys.exit(1)

    image_files.sort()

    print(f"Analyzing {len(image_files)} files:")
    for f in image_files:
        print(f"  {f.name}")

    analyze_spatial_distribution(image_files)
