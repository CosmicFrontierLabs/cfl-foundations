#!/usr/bin/env python3
"""Test using the actual PIPython library."""

import sys
sys.path.insert(0, '/home/meawoppl/repos/meter-sim/ext_ref/PIPython/PIPython/extracted/PIPython-2.10.2.1')

from pipython import GCSDevice, pitools

def main():
    print("Creating GCSDevice...")
    pidevice = GCSDevice()

    print("Enumerating USB devices...")
    devices = pidevice.EnumerateUSB()
    print(f"Found devices: {devices}")

    if not devices:
        print("No devices found!")
        return

    # Get first device info
    first = devices[0]
    print(f"Connecting to: {first}")

    # Connect
    pidevice.ConnectUSB(first)

    print(f"Connected! IDN: {pidevice.qIDN()}")
    print(f"Axes: {pidevice.axes}")

    # Query positions
    positions = pidevice.qPOS()
    print(f"Positions: {positions}")

    # Close
    pidevice.CloseConnection()
    print("Closed connection")

    # Try reconnecting
    print("\n--- Reconnecting ---")
    pidevice2 = GCSDevice()
    pidevice2.ConnectUSB(first)
    print(f"Reconnected! IDN: {pidevice2.qIDN()}")
    pidevice2.CloseConnection()
    print("Done!")

if __name__ == "__main__":
    main()
