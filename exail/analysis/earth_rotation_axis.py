#!/usr/bin/env python3
"""Determine Earth's rotation axis from gyro data."""

import argparse
import numpy as np
import pandas as pd


def load_data(csv_path: str) -> pd.DataFrame:
    """Load CSV and filter to data rows only."""
    df = pd.read_csv(csv_path, low_memory=False)
    return df[df["type"] == "data"].copy()


def compute_angular_rates(df: pd.DataFrame):
    """Compute mean angular rates on each axis from filtered data."""
    # TimeTag ticks at 2 kHz (0.5ms per tick)
    TIMETAG_HZ = 2000.0
    TICK_PERIOD_SEC = 1.0 / TIMETAG_HZ

    # Get timestamp data
    timestamps = df["gyro_time"].values

    # Extract lower 16 bits (the part that increments)
    lower_bits = (timestamps % 65536).astype(np.int64)

    # Compute time differences with u16 rollover correction
    time_diffs = np.diff(lower_bits)
    time_diffs = np.where(time_diffs < 0, time_diffs + 65536, time_diffs)

    # Total time in ticks and seconds
    total_ticks = np.sum(time_diffs)
    total_time_sec = total_ticks * TICK_PERIOD_SEC

    # Get angle data for filtered measurements (in arcseconds)
    fil_x = pd.to_numeric(df["fil_ang_x_arcsec"], errors='coerce').values
    fil_y = pd.to_numeric(df["fil_ang_y_arcsec"], errors='coerce').values
    fil_z = pd.to_numeric(df["fil_ang_z_arcsec"], errors='coerce').values

    # Filter out NaN values
    valid_x = ~np.isnan(fil_x)
    valid_y = ~np.isnan(fil_y)
    valid_z = ~np.isnan(fil_z)

    # Compute total angular change
    total_x = np.sum(np.diff(fil_x[valid_x]))
    total_y = np.sum(np.diff(fil_y[valid_y]))
    total_z = np.sum(np.diff(fil_z[valid_z]))

    n_samples = len(df)

    # Compute sample rate
    sample_rate_hz = n_samples / total_time_sec if total_time_sec > 0 else 0
    median_tick = np.median(time_diffs[time_diffs > 0])

    print(f"\n=== Timing Information ===")
    print(f"Total samples: {n_samples:,}")
    print(f"Total time: {total_time_sec:.2f} seconds ({total_time_sec/60:.2f} minutes)")
    print(f"Sample rate: {sample_rate_hz:.1f} Hz")
    print(f"Sample period: {1000.0/sample_rate_hz:.3f} ms")
    print(f"Median ticks between samples: {median_tick:.1f} ({median_tick*TICK_PERIOD_SEC*1000:.3f} ms)")

    print("\n=== Total Angular Accumulation (arcseconds) ===")
    print(f"X-axis: {total_x:+.1f} arcsec")
    print(f"Y-axis: {total_y:+.1f} arcsec")
    print(f"Z-axis: {total_z:+.1f} arcsec")

    # Compute rotation rates
    rate_x_arcsec_per_sec = total_x / total_time_sec
    rate_y_arcsec_per_sec = total_y / total_time_sec
    rate_z_arcsec_per_sec = total_z / total_time_sec

    print("\n=== Measured Rotation Rates ===")
    print(f"X: {rate_x_arcsec_per_sec:+.3f} arcsec/sec")
    print(f"Y: {rate_y_arcsec_per_sec:+.3f} arcsec/sec")
    print(f"Z: {rate_z_arcsec_per_sec:+.3f} arcsec/sec")

    # Compute magnitude
    magnitude = np.sqrt(rate_x_arcsec_per_sec**2 + rate_y_arcsec_per_sec**2 + rate_z_arcsec_per_sec**2)
    print(f"\nTotal magnitude: {magnitude:.3f} arcsec/sec")

    # Earth rotates at 15.041 arcsec/second (sidereal rate)
    earth_rate = 15.041
    print(f"Earth's rotation rate: {earth_rate:.3f} arcsec/sec")

    error_percent = ((magnitude - earth_rate) / earth_rate) * 100
    print(f"Difference: {error_percent:+.1f}%")

    # Find which axis has the largest rotation rate
    totals = [abs(rate_x_arcsec_per_sec), abs(rate_y_arcsec_per_sec), abs(rate_z_arcsec_per_sec)]
    max_idx = np.argmax(totals)
    axis_names = ['X', 'Y', 'Z']
    rate_values = [rate_x_arcsec_per_sec, rate_y_arcsec_per_sec, rate_z_arcsec_per_sec]

    print(f"\n=== Earth's Rotation Vector (device frame) ===")
    print(f"Primary axis: {axis_names[max_idx]}-axis ({rate_values[max_idx]:+.3f} arcsec/sec)")

    # Compute unit vector
    if magnitude > 0:
        unit_x = rate_x_arcsec_per_sec / magnitude
        unit_y = rate_y_arcsec_per_sec / magnitude
        unit_z = rate_z_arcsec_per_sec / magnitude

        print(f"\nUnit vector:")
        print(f"  X: {unit_x:+.4f}")
        print(f"  Y: {unit_y:+.4f}")
        print(f"  Z: {unit_z:+.4f}")

        # Compute angle from each axis
        print(f"\nAngle from device axes:")
        if abs(unit_x) <= 1:
            angle_from_x = np.arccos(abs(unit_x)) * 180 / np.pi
            print(f"  From X-axis: {angle_from_x:.1f}°")
        if abs(unit_y) <= 1:
            angle_from_y = np.arccos(abs(unit_y)) * 180 / np.pi
            print(f"  From Y-axis: {angle_from_y:.1f}°")
        if abs(unit_z) <= 1:
            angle_from_z = np.arccos(abs(unit_z)) * 180 / np.pi
            print(f"  From Z-axis: {angle_from_z:.1f}°")

        # Compute latitude assuming Z-axis points toward local gravity (down)
        # WGS84 ellipsoid: Local vertical (plumb line) differs from geocentric radius
        # The angle between them is the "angle of the vertical" or "deflection"
        # For WGS84: flattening f = 1/298.257223563
        print(f"\n=== Estimated Latitude ===")
        print(f"Assumption: Z-axis points toward local gravity (downward)")

        # WGS84 parameters
        WGS84_FLATTENING = 1.0 / 298.257223563
        WGS84_ECCENTRICITY_SQ = 2 * WGS84_FLATTENING - WGS84_FLATTENING**2

        # First approximation: geodetic latitude from Z-component
        # Earth's rotation axis in local vertical frame:
        # - At geodetic latitude φ, the local vertical makes angle φ with equatorial plane
        # - But rotation axis makes angle φ with equator, so it makes (90° - φ) with vertical
        # - Z-component (pointing down) = sin(φ) in spherical case

        # For ellipsoid, we need to account for the difference between:
        # - Geodetic latitude φ (angle of local vertical from equator)
        # - Geocentric latitude φ' (angle of radius vector from equator)
        # The rotation axis is parallel to geometric axis, not aligned with local vertical

        # The Z-component of rotation vector in local vertical frame:
        # unit_z ≈ sin(φ) for sphere
        # But for ellipsoid: unit_z = sin(φ_geocentric)
        # where φ_geocentric = arctan((1-e²) * tan(φ_geodetic))

        # Iterate to find geodetic latitude
        # Start with spherical approximation
        lat_geocentric_rad = np.arcsin(unit_z)

        # Convert geocentric to geodetic latitude (iterative)
        # tan(φ_geodetic) = tan(φ_geocentric) / (1 - e²)
        lat_geodetic_rad = np.arctan(np.tan(lat_geocentric_rad) / (1 - WGS84_ECCENTRICITY_SQ))

        # Refine using proper formula
        # For better accuracy, iterate the relationship:
        # φ_geocentric = arctan((1-e²) * tan(φ_geodetic))
        for _ in range(5):
            lat_geocentric_check = np.arctan((1 - WGS84_ECCENTRICITY_SQ) * np.tan(lat_geodetic_rad))
            # Adjust geodetic latitude based on error
            lat_geodetic_rad = np.arctan(np.tan(lat_geocentric_rad) / (1 - WGS84_ECCENTRICITY_SQ))

        lat_geodetic_deg = lat_geodetic_rad * 180 / np.pi
        lat_geocentric_deg = lat_geocentric_rad * 180 / np.pi

        # Compute the difference (angle of the vertical)
        vertical_deflection = lat_geodetic_deg - lat_geocentric_deg

        print(f"\nSpherical approximation: {lat_geocentric_deg:.2f}°")
        print(f"WGS84 geodetic latitude: {lat_geodetic_deg:.2f}°")
        print(f"Correction due to ellipsoid: {vertical_deflection:+.2f}°")
        print(f"Hemisphere: {'North' if lat_geodetic_deg > 0 else 'South'}")

        # The horizontal component tells us about azimuth
        horiz_magnitude = np.sqrt(unit_x**2 + unit_y**2)
        print(f"\nHorizontal component magnitude: {horiz_magnitude:.4f}")
        print(f"  (should equal cos(geocentric_lat) = {np.cos(lat_geocentric_rad):.4f})")

        if horiz_magnitude > 0:
            # Azimuth of the north direction in device frame (angle from X-axis in XY plane)
            azimuth_rad = np.arctan2(unit_y, unit_x)
            azimuth_deg = azimuth_rad * 180 / np.pi
            print(f"\nNorth direction in device XY plane:")
            print(f"  Azimuth from X-axis: {azimuth_deg:.1f}°")
            print(f"  (North points toward: X={unit_x/horiz_magnitude:+.4f}, Y={unit_y/horiz_magnitude:+.4f})")


def main():
    parser = argparse.ArgumentParser(description="Determine Earth's rotation axis from gyro data")
    parser.add_argument("csv_file", help="Input CSV file from parse_dump")
    args = parser.parse_args()

    print(f"Loading {args.csv_file}...")
    df = load_data(args.csv_file)
    print(f"Loaded {len(df):,} data records")

    compute_angular_rates(df)


if __name__ == "__main__":
    main()
