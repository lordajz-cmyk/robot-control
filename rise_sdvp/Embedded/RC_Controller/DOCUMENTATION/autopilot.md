# Autopilot Module Documentation

## Overview

The `autopilot.c` module implements autonomous navigation functionality for the RC Controller system. It enables the vehicle to follow predefined routes consisting of waypoints (ROUTE_POINT structures) by calculating appropriate steering angles and speeds.

## Key Features

- **Route Management**: Add, remove, clear, and replace route points
- **Dynamic Radius Calculation**: Adjusts turning radius based on current speed
- **Path Following**: Uses circle-line intersection algorithms for accurate path following
- **Multiple Navigation Modes**: Supports both time-based and distance-based navigation
- **Route Repetition**: Can repeatedly follow the same route
- **Speed Override**: Allows manual speed control while autopilot is active
- **Positioning Integration**: Works with GPS and UWB (Ultra-Wideband) positioning systems
- **Hydraulic Control**: Supports hydraulic actuator control based on route attributes (when HAS_HYDRAULIC_DRIVE is enabled)
- **Differential Steering**: Supports differential steering systems (when HAS_DIFF_STEERING is enabled)

## Module Architecture

### Thread Structure

The autopilot runs in a dedicated thread (`ap_thread`) at a frequency of **100Hz** (defined by `AP_HZ`). This high frequency ensures smooth and responsive path following.

The thread performs the following operations in each cycle:
1. Checks if autopilot is active
2. Retrieves current vehicle position
3. Handles attribute-based control (positioning, hydraulics)
4. Calculates path following using circle-line intersection
5. Computes steering angle and speed
6. Sends control commands to motor control system

### Data Structures

#### ROUTE_POINT
The fundamental waypoint structure containing:
- `px`, `py`, `pz`: Position coordinates (meters)
- `speed`: Desired speed at this point (m/s)
- `time`: Timestamp for time-based navigation (milliseconds since midnight)
- `attributes`: Flags for special behavior (positioning mode, hydraulic control, etc.)

#### Route Storage
- Routes are stored in a circular buffer: `m_route[AP_ROUTE_SIZE]`
- Maximum route size: **2000 points** (defined by `AP_ROUTE_SIZE` in `conf_general.h`)
- Uses circular buffer indices: `m_point_now` (current point), `m_point_last` (last point)

### Main Components

#### State Management
- `m_is_active`: Master enable/disable flag for autopilot
- `m_is_route_started`: Indicates if route following has begun
- `m_route_end`: Flag indicating end of route has been reached
- `m_route_left`: Number of points remaining in the route

#### Path Following Parameters
- `m_rad_now`: Current turning radius (meters)
- `m_rp_now`: Current target point being followed
- `m_route_look_ahead`: Number of points to look ahead (default: 8)
- `m_en_dynamic_rad`: Enable/disable dynamic radius calculation
- `m_en_angle_dist_comp`: Enable/disable angle-distance compensation

#### Speed Control
- `m_override_speed`: Manual speed override value (m/s)
- `m_is_speed_override`: Flag indicating if speed override is active
- `m_sync_rx`: Synchronization flag for time-based navigation

## Public API

### Initialization

```c
void autopilot_init(void);
```
Initializes the autopilot module, registers terminal commands, and starts the autopilot thread.

### Route Management

```c
bool autopilot_add_point(ROUTE_POINT *p, bool first);
```
Adds a point to the current route.
- `p`: Pointer to the ROUTE_POINT to add
- `first`: True if this is the first point in a packet (used for duplicate detection)
- Returns: True if point was added, false if it was a duplicate

```c
void autopilot_remove_last_point(void);
```
Removes the most recently added point from the route.

```c
void autopilot_clear_route(void);
```
Clears all points from the current route and resets state.

```c
bool autopilot_replace_route(ROUTE_POINT *p);
```
Replaces the current route with a new point. Intelligently handles time-based routing.
- If autopilot is inactive: clears route and adds new point
- If autopilot is active: replaces appropriate part of route based on timestamps

```c
void autopilot_sync_point(int32_t point, int32_t time, int32_t min_time_diff);
```
Synchronizes route points to ensure vehicle reaches a specific point at a specific time.
- `point`: Index of target point
- `time`: Time (ms) when point should be reached
- `min_time_diff`: Minimum time difference for valid synchronization

### State Control

```c
void autopilot_set_active(bool active);
```
Activates or deactivates the autopilot.

```c
bool autopilot_is_active(void);
```
Returns true if autopilot is currently active.

```c
void autopilot_reset_state(void);
```
Resets all autopilot state variables.

### Information Retrieval

```c
int autopilot_get_route_len(void);
```
Returns the number of points in the current route.

```c
int autopilot_get_point_now(void);
```
Returns the index of the current point being followed.

```c
int autopilot_get_route_left(void);
```
Returns the number of points remaining in the route.

```c
ROUTE_POINT autopilot_get_route_point(int ind);
```
Retrieves a specific route point by index.
- `ind`: Index of the point to retrieve
- Returns: The ROUTE_POINT at the specified index, or empty point if invalid

```c
float autopilot_get_rad_now(void);
```
Returns the current turning radius in meters (-1.0 = invalid).

```c
void autopilot_get_goal_now(ROUTE_POINT *rp);
```
Retrieves the current goal point the autopilot is following.
- `rp`: Pointer to store the goal point

### Speed Control

```c
void autopilot_set_speed_override(bool is_override, float speed);
```
Overrides the route-defined speed with a fixed speed.
- `is_override`: True to enable override, false to use route speed
- `speed`: Speed in m/s (ignored if is_override is false)

```c
void autopilot_set_motor_speed(float speed);
```
Directly sets the motor speed (m/s). Respects motor disable configuration.

```c
float autopilot_get_steering_scale(void);
```
Returns the steering scale factor based on current speed.
- At 0 m/s: factor = 1.0 (full steering)
- At 10 m/s: factor ≈ 0.5 (reduced steering for stability)
- Formula: `1.0 / (1.0 + speed * 0.05)^2`

### Differential Steering Support

```c
void autopilot_set_turn_rad(float rad);
```
Sets the turning radius from front wheel feedback (for differential steering).
- `rad`: Turning radius in meters
- Only available when `HAS_DIFF_STEERING` is defined

## Path Following Algorithm

The autopilot uses a **circle-line intersection** algorithm for path following:

1. **Dynamic Radius Calculation**: 
   - If `m_en_dynamic_rad` is true: `m_rad_now = fabs(main_config.ap_rad_time_ahead * current_speed)`
   - Minimum radius is clamped to `main_config.ap_base_rad`
   - If `m_en_dynamic_rad` is false: uses fixed `main_config.ap_base_rad`

2. **Look-Ahead**:
   - Looks `m_route_look_ahead` points ahead along the route
   - Finds circle intersections between vehicle position and route segments

3. **Target Point Selection**:
   - If circle intersection found: use intersection point
   - If no intersection: use closest point on route

4. **Steering Calculation**:
   - Uses `steering_angle_to_point()` to calculate required steering angle
   - Applies speed-dependent scaling with `autopilot_get_steering_scale()`
   - Limits steering to `main_config.vehicle.steering_max_angle_rad`

5. **Speed Calculation**:
   - **Time Mode** (`main_config.ap_mode_time`): Calculates speed to reach points at specified times
   - **Distance Mode**: Interpolates speed between closest route points
   - Applies speed override if enabled
   - Clamps to `main_config.ap_max_speed`

## Attribute-Based Control

The autopilot supports attribute-based control for:

### Positioning Mode
Controlled by `ATTR_POSITIONING_MASK`:
- `ATTR_POSITIONING_DEFAULT`: Use default positioning (GPS)
- `ATTR_POSITIONING_UWB`: Use Ultra-Wideband positioning
- Default: Use configured positioning system

### Hydraulic Control
Controlled by `ATTR_HYDRAULIC_MASK` (when `HAS_HYDRAULIC_DRIVE` is defined):
- `ATTR_HYDRAULIC_FRONT_UP/DOWN`: Move front hydraulic up/down
- `ATTR_HYDRAULIC_REAR_UP/DOWN`: Move rear hydraulic up/down
- `ATTR_HYDRAULIC_EXTRA_OUT/IN`: Move extra hydraulic out/in
- Default: Stop all hydraulic movement

Note: Only one hydraulic valve can be actuated at a time using attributes.

## Configuration Options

The autopilot behavior is controlled by configuration parameters in `main_config`:

- `vehicle.use_uwb_pos`: Use UWB positioning instead of GPS
- `vehicle.disable_motor`: Disable motor control
- `vehicle.steering_max_angle_rad`: Maximum steering angle in radians
- `vehicle.steering_range`: Steering servo range
- `vehicle.steering_center`: Steering servo center position
- `vehicle.axis_distance`: Distance between vehicle axes (for turning radius calculation)
- `ap_max_speed`: Maximum allowed speed (m/s)
- `ap_base_rad`: Base turning radius (meters)
- `ap_rad_time_ahead`: Time-ahead factor for dynamic radius calculation
- `ap_repeat_routes`: Enable route repetition
- `ap_time_add_repeat_ms`: Time to add for route repetition (milliseconds)
- `ap_mode_time`: Time-based navigation mode (0=disabled, 1=basic, 2=advanced)

## Terminal Commands

The autopilot registers several terminal commands for debugging and configuration:

| Command | Description | Arguments |
|---------|-------------|-----------|
| `ap_state` | Print autopilot state variables | None |
| `ap_print_closest` | Enable/disable closest point printing | `[print_rate]` (0=disabled, n=every n iterations) |
| `ap_dynamic_rad` | Enable/disable dynamic radius | `[enabled]` (0=disabled, 1=enabled) |
| `ap_ang_dist_comp` | Enable/disable angle-distance compensation | `[enabled]` (0=disabled, 1=enabled) |
| `ap_set_look_ahead` | Set look-ahead distance in points | `[points]` |

## Thread Safety

The autopilot uses a mutex (`m_ap_lock`) for thread-safe access to shared data. Most public functions lock the mutex before accessing internal state and unlock it when done.

Note: `autopilot_set_speed_override()` does not use the mutex, which is likely intentional for performance in time-critical code.

## Memory Layout

- Route storage: Located in RAM4 section (`__attribute__((section(".ram4")))`)
- Thread stack: 2048 bytes (`ap_thread_wa`)
- Maximum route size: 2000 points × sizeof(ROUTE_POINT) ≈ 2000 × 24 = 48KB

## Dependencies

The autopilot module depends on:
- `ch.h`, `hal.h`: ChibiOS/RT headers for threading and synchronization
- `pos.h`, `pos_uwb.h`: Positioning system interfaces
- `utils.h`: Utility functions (distance calculations, circle-line intersection)
- `motor_control.h`: Motor control interface
- `bldc_interface.h`: BLDC motor interface
- `commands.h`, `terminal.h`: Command and terminal interfaces
- `comm_can.h`: CAN communication interface
- `attributes_masks.h`: Attribute flag definitions
- `servo_simple.h`: Servo control interface

## Usage Example

```c
// Initialize autopilot
autopilot_init();

// Create a route point
ROUTE_POINT point;
point.px = 10.0;  // X coordinate (meters)
point.py = 5.0;   // Y coordinate (meters)
point.speed = 2.0; // Speed (m/s)
point.time = 0;   // Timestamp (0 = use distance-based mode)
point.attributes = 0; // No special attributes

// Add point to route
autopilot_add_point(&point, true);

// Activate autopilot
autopilot_set_active(true);

// Later, deactivate autopilot
autopilot_set_active(false);

// Clear route
autopilot_clear_route();
```

## Implementation Notes

1. **Circular Buffer**: The route uses a circular buffer, so indices wrap around using modulo arithmetic.

2. **Time Handling**: Times are in milliseconds since midnight (0-86400000). The `utils_time_before()` function handles midnight wrap-around.

3. **Coordinate System**: The coordinate system uses meters for all distance calculations.

4. **Angle Units**: Angles are in radians for internal calculations, but the position system may use degrees.

5. **Speed Units**: All speeds are in meters per second (m/s).

6. **Thread Priority**: The autopilot thread runs at `NORMALPRIO` priority.

## Debugging

The autopilot includes several debugging features:

- `iDebug` variable: Used for conditional debug output
- `m_print_closest_point`: When enabled, prints distance to closest point, speed, yaw, and radius
- Terminal commands for inspecting and modifying autopilot state

Debug values can be printed using `commands_printf()` for troubleshooting.

## Future Enhancements

Based on commented code and TODOs:
- Emergency stop event handling (currently commented out)
- Differential steering integration (partially implemented)
- Hydraulic system enhancements
- Improved path following algorithms
- Better handling of route synchronization
