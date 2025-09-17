#!/usr/bin/env python3

import io
import json
from dataclasses import dataclass
from typing import Optional, List, Tuple
from PIL import Image
import numpy as np
import requests


DEFAULT_HIL_SERVER = "192.168.15.229"
DEFAULT_PORT = 9999


@dataclass
class FrameStats:
    total_frames: int
    current_fps: float
    errors: int
    fpga_temp_c: Optional[float]
    pcb_temp_c: Optional[float]
    timestamp_sec: int
    timestamp_usec: int
    histogram: List[int]


@dataclass
class RawFrameData:
    data: np.ndarray
    width: int
    height: int
    format: str
    timestamp_sec: int
    timestamp_usec: int


@dataclass
class JpegFrameData:
    image: Image.Image
    timestamp_sec: int
    timestamp_usec: int


class HilClient:
    def __init__(self, host: str = DEFAULT_HIL_SERVER, port: int = DEFAULT_PORT):
        self.base_url = f"http://{host}:{port}"
        self.timeout = 10.0

    def get_stats(self) -> FrameStats:
        """Get current frame statistics from the HIL server."""
        response = requests.get(f"{self.base_url}/stats", timeout=self.timeout)
        response.raise_for_status()

        data = response.json()
        return FrameStats(
            total_frames=data["total_frames"],
            current_fps=data["current_fps"],
            errors=data["errors"],
            fpga_temp_c=data["fpga_temp_c"] if data["fpga_temp_c"] != "null" else None,
            pcb_temp_c=data["pcb_temp_c"] if data["pcb_temp_c"] != "null" else None,
            timestamp_sec=data["timestamp_sec"],
            timestamp_usec=data["timestamp_usec"],
            histogram=data["histogram"]
        )

    def get_raw_frame(self) -> RawFrameData:
        """Get raw 16-bit frame data from the HIL server."""
        response = requests.get(f"{self.base_url}/raw", timeout=self.timeout)
        response.raise_for_status()

        # Extract metadata from headers
        width = int(response.headers["X-Frame-Width"])
        height = int(response.headers["X-Frame-Height"])
        format_str = response.headers["X-Frame-Format"]
        timestamp_sec = int(response.headers["X-Timestamp-Sec"])
        timestamp_usec = int(response.headers["X-Timestamp-Usec"])

        # Convert raw bytes to numpy array (16-bit unsigned)
        raw_bytes = response.content
        data = np.frombuffer(raw_bytes, dtype=np.uint16).reshape((height, width))

        return RawFrameData(
            data=data,
            width=width,
            height=height,
            format=format_str,
            timestamp_sec=timestamp_sec,
            timestamp_usec=timestamp_usec
        )

    def get_jpeg_frame(self) -> JpegFrameData:
        """Get JPEG-encoded frame from the HIL server."""
        response = requests.get(f"{self.base_url}/jpeg", timeout=self.timeout)
        response.raise_for_status()

        # Extract timestamp from headers
        timestamp_sec = int(response.headers["X-Timestamp-Sec"])
        timestamp_usec = int(response.headers["X-Timestamp-Usec"])

        # Decode JPEG image
        image = Image.open(io.BytesIO(response.content))

        return JpegFrameData(
            image=image,
            timestamp_sec=timestamp_sec,
            timestamp_usec=timestamp_usec
        )

    def get_camera_status_page(self) -> str:
        """Get the HTML camera status page."""
        response = requests.get(f"{self.base_url}/", timeout=self.timeout)
        response.raise_for_status()
        return response.text

    def check_connection(self) -> bool:
        """Check if HIL server is reachable."""
        try:
            response = requests.get(f"{self.base_url}/stats", timeout=2.0)
            return response.status_code == 200
        except (requests.RequestException, requests.Timeout):
            return False


def main():
    """Example usage of the HIL client."""
    client = HilClient()

    print(f"Connecting to HIL server at {client.base_url}...")

    if not client.check_connection():
        print("Failed to connect to HIL server!")
        return

    print("Connection successful!\n")

    # Get and display stats
    print("=== Frame Statistics ===")
    stats = client.get_stats()
    print(f"Total frames: {stats.total_frames}")
    print(f"Current FPS: {stats.current_fps:.1f}")
    print(f"FPGA temp: {stats.fpga_temp_c:.1f}°C" if stats.fpga_temp_c else "FPGA temp: N/A")
    print(f"PCB temp: {stats.pcb_temp_c:.1f}°C" if stats.pcb_temp_c else "PCB temp: N/A")
    print(f"Timestamp: {stats.timestamp_sec}.{stats.timestamp_usec:06d}")
    print(f"Histogram bins (first 10): {stats.histogram[:10]}")

    # Get raw frame
    print("\n=== Raw Frame ===")
    raw_frame = client.get_raw_frame()
    print(f"Resolution: {raw_frame.width}x{raw_frame.height}")
    print(f"Format: {raw_frame.format}")
    print(f"Data shape: {raw_frame.data.shape}")
    print(f"Data range: [{raw_frame.data.min()}, {raw_frame.data.max()}]")
    print(f"Mean value: {raw_frame.data.mean():.1f}")

    # Get JPEG frame
    print("\n=== JPEG Frame ===")
    jpeg_frame = client.get_jpeg_frame()
    print(f"Image size: {jpeg_frame.image.size}")
    print(f"Image mode: {jpeg_frame.image.mode}")
    print(f"Timestamp: {jpeg_frame.timestamp_sec}.{jpeg_frame.timestamp_usec:06d}")

    # Optionally save the JPEG
    # jpeg_frame.image.save("captured_frame.jpg")
    # print("Saved frame to captured_frame.jpg")


if __name__ == "__main__":
    main()