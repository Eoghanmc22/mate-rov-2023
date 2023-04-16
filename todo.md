# Queue

- Leveling ui
- Profile robot code
- Make things customizable at run time
- Sensor fusion init
- Depth control
- Remap joysticks

# Important Things

- [ ] Sensors
  - [-] Sensor fusion
    - [H] Use last sensor data as stating position
    - [ ] Magnetometers?
  - [-] Make sure sensors power down, etc on drop
  - [-] Make sure sensors do a reset on init
- [ ] Control
  - [x] Pid stuff
  - [H] Leveling control
    - [ ] Tuning
  - [M] Depth control
  - [ ] Speeds larger than 1
  - [ ] Remap inputs to square joystick
- [ ] Debugging
  - [?] Better logging
  - [ ] Make important settings editable in real time
- [ ] UI
  - [ ] Leveling ui
    - [ ] pid editing
    - [ ] Display automous modes in bottom status ui?
  - [ ] Window to view and edit global store
    - [ ] Max motor speed, 
- [ ] Misc
  - [ ] Put more stuff in the global store, model after how pid config is handled for leveling
  - [H] Fix nic sleep on laptop
  - [M] Investigate high cpu usage on pi

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

