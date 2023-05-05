# Queue

- Support for absolute movement
- Sensor fusion init
- Improve user input
- Tune pids
- Autonomous stuff
- Make things customizable at run time
- handle more unhappy paths, especially if let statements

# Important Things

- [ ] Motors
  - [ ] Fix drop impl
- [ ] Sensors
  - [-] Sensor fusion
    - [H] Use last sensor data as stating position
  - [-] Make sure sensors power down, etc on drop
  - [-] Make sure sensors do a reset on init
- [ ] Control
  - [M] Depth control
- [ ] Debugging
  - [?] Better logging
  - [ ] Make important settings editable in real time
- [ ] UI
  - [ ] Better pilot controls
    - [ ] Remap inputs to square joystick
    - [ ] 2 controller support
    - [ ] Dedicated controls for servo
    - [ ] Servo velocity control

# Low Priority

- [ ] Sensors
  - [-] Magnetometers
    - [ ] Other compass as well
    - [ ] Calibration
  - [?] Check data ready flags in read frame code
- [ ] Control
  - [ ] OpenCV control
- [ ] Debugging
  - [?] Better & more error notifs in ui
  - [?] Tests
- [ ] Ui
  - [ ] Errors in uis should be handled better
  - [ ] Reduce usage of clone
  - [ ] Better video view
  - [ ] Improve notifications
    - [ ] Animations
    - [ ] Color
    - [ ] Timer
  - [ ] Visualize our data
  - [ ] Use the LEDs on the navigator better
  - [M] Make a README.md
- [ ] Misc
  - [H] Focus loss breaks things
  - [ ] `Updater` could be replaced with a method on `Robot` or a function of `World`
  - [ ] See if more things should in fixed update schedule
  - [ ] Remove any debugging prints
  - [ ] Some calls to log_error should be replaced with proper handling and or the opposite
  - [ ] Surface network system has too much responsibility
  - [ ] Image pi
  - [ ] Systemd service
  - [ ] Surface prints errors on shutdown
  - [L] Investigate high cpu usage on pi
  - [ ] Put more stuff in the global store, model after how pid config is handled for leveling

