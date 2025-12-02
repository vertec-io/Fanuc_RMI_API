# How to start the Web Based FANUC Controller

First, in one terminal, you need to start the web server but running the following command from the root directory of the project:
```bash
cargo run -p web_server
```

Then, in a separate terminal, you need to build the web app by running the following command from the root directory of the project:
```bash
cd web_app && trunk build --release
```

Finally, you can serve the web app by running the following command from the root directory of the project:
```bash
cd web_app && python -m http.server 8080
```

Then you can navigate to `http://localhost:8080` in your browser to access the web app.

If you want to run the simulator instead of connecting to a real robot, start the simulator with the following command from the root directory of the project:
```bash
cargo run -p sim -- --realtime
```

# How to Configure the Web Based FANUC Controller

The web based FANUC controller can be configured to connect to multiple differently configured FANUC robots in the local database. 

To create a new robot connection, go to the Settings page and click on the "Add Robot" button. Fill in the details of the robot you want to connect to and click "Save". The new robot connection will be added to the list of saved robots.

Be sure to configure the correct Configuration settings for your robot so that the jog commands and UFrame and UTool settings match your robot's configuration. These include the following fields:

## Default Coordinate Frame and Tool
- UToolNumber
- UFrameNumber

## Default Jog Settings
- Front
- Up
- Left
- Flip
- Turn4
- Turn5
- Turn6

## Default Jog Speed and Termination Type
- Speed
- TermType --> Recommend using `FINE` for jog commands

## Default Rotation
- W --> Roll
- P --> Pitch
- R --> Yaw

# Create a Toolpath Program

You can create a new toolpath program by going to the "Programs" page and clicking on the "New Program" button. You can then add a name and description for the program and upload a CSV file containing the toolpath data. The CSV file should have the following format:

```csv
x,y,z,speed
100.0,200.0,300.0,50
150.0,200.0,300.0,100
```

It will use the default rotation, and termination type specified in the robot configuration and automatically set the termination type of the last instruction to `FINE`.

# Connect a Robot, Take Control, and Operate the Robot

In order to connect to a given robot and control it, you must first have system control. On the Dashboard page, select the "Request Control" button. This will put the system in control and allow you to send commands to the robot.

In the Header at the top right there is a quick settings Gear icon where you can see the list of saved robot connections. Click on the robot you want to connect to and click "Connect". This will connect to the robot and apply the configuration settings for that robot.

Once you have control of the robot, you can jog it, send it specific commands, and run programs. You can click the "Initialize" and "Abort" commands to get the program into a state of execution (i.e. abort previously running TP Programs and initialize a new TP program session). You can click the "Reset" button to clear errors and try to recover from a fault state.

You can use the Command Composer to create and send specific commands to the robot. Previously sent commands will be available in the recent commands list and can be resent by clicking on the command in the list.

# Loading and Executing a Program

Toolpaths are set up as programs and must be created in the Programs menu before they can be loaded for execution on the robot. Once you have a program configured, you can click on the "Load" button on the Program section of the Control tab in the Dashboard and select the program you want to load. This will load the program into the robot's TP program session and you will be able to see the program in the Program Visualizer. 

When you're ready, you can click "Run" to start the program. The robot will move to the first position in the program and start executing the program. You can pause and resume the program at any time. You can also stop the program at any time. When you stop the program, it will clear the program from the robot's TP program session and you will need to load the program again in order to run it.



