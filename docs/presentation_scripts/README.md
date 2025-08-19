# ICP Animation Scripts

This directory contains animation scripts for visualizing the Iterative Closest Point (ICP) algorithm using Manim.

## Installation

### Prerequisites
- Python 3.8+
- uv (Python package manager)

### Setup

1. Install Manim Community Edition using uv:
```bash
uv pip install manim
```

Or if you prefer pip:
```bash
pip install manim
```

## Files

- `icp_animation.py` - Main animation script containing two scenes:
  - `ICPAnimation` - Basic ICP algorithm demonstration
  - `ICPWithNoise` - ICP with noisy data and outliers

## Running the Animations

### Basic Usage

```bash
# Low quality preview (480p, 15fps) - fastest rendering
uv run manim -pql icp_animation.py ICPAnimation

# Medium quality (720p, 30fps)
uv run manim -pqm icp_animation.py ICPAnimation

# High quality (1080p, 60fps)
uv run manim -pqh icp_animation.py ICPAnimation

# 4K quality (2160p, 60fps) - slowest rendering
uv run manim -pqk icp_animation.py ICPAnimation
```

### Flags
- `-p` : Preview the animation after rendering
- `-q` : Quality flag (followed by l/m/h/k for low/medium/high/4K)
- `-s` : Save the last frame as an image

### Run the noise animation
```bash
uv run manim -pql icp_animation.py ICPWithNoise
```

## Output

Rendered videos are saved to:
```
media/videos/icp_animation/<quality>/ICPAnimation.mp4
```

Where `<quality>` is one of:
- `480p15` - Low quality
- `720p30` - Medium quality  
- `1080p60` - High quality
- `2160p60` - 4K quality

## What the Animation Shows

The ICP animation demonstrates:
1. **Finding nearest neighbors** - Yellow lines show point correspondences
2. **Computing optimal transformation** - Calculate rotation and translation
3. **Applying transformation** - Move source points closer to target
4. **Iteration** - Repeat until convergence

Features visualized:
- Blue dots: Source point cloud
- Red dots: Target points to align to
- Orange dots: Extra points (outliers) that ICP handles
- Yellow lines: Current nearest neighbor correspondences
- Text indicators: Current iteration, error metric, and algorithm step

## Customization

Edit `icp_animation.py` to:
- Change point distributions (modify `create_star_pattern()`)
- Adjust transformation magnitude (modify `rotation` and `translation` values)
- Add/remove outlier points (modify `extra_points` array)
- Change colors, sizes, or animation speeds

## Troubleshooting

If you get module import errors:
```bash
# Make sure manim is installed
pip list | grep manim

# If using uv, make sure you're in the right environment
uv pip list | grep manim
```

If rendering is too slow:
- Use lower quality settings (-pql instead of -pqh)
- Reduce the number of iterations in the animation
- Disable preview (-p flag) to avoid opening video player

## Notes

- First run may take longer as Manim caches some computations
- The `media/` folder can grow large with multiple renders - clean periodically
- 4K rendering can take several minutes depending on your hardware