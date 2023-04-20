# ancs-notify

Edited /etc/bluetooth/main.conf to set "ControllerMode = bredr"

If you get an error "Failed to register advertisement"

journalctl -u bluetooth

```
Apr 15 12:05:04 idril bluetoothd[1056]: src/advertising.c:add_client_complete() Failed to add advertisement: Invalid Parameters (0x0d)
```

```
systemctl restart bluetooth
```


## Debug BlueZ using D-bus

```
dbus-monitor --system "type='signal',sender='org.bluez'"
```

## References

- https://github.com/pop-os/cosmic-applets/blob/master_jammy/cosmic-applet-bluetooth/src/bluetooth.rs
- pairing agent https://github.com/khvzak/bluez-tools/blob/master/src/bt-agent.c
- ancs https://github.com/S-March/smarchWatch_Public/blob/834e12664924f2e5c0c4c88b5488b8995cc20acf/Software/smarchWatch_DA14683/DA1468x_SDK_1.0.14.1081/DA1468x_DA15xxx_SDK_1.0.14.1081/projects/dk_apps/ble_profiles/smarchWatch/ancs_task.c#L719
- c https://github.com/weliem/bluez_inc/blob/main/binc/advertisement.c

https://developer.apple.com/library/archive/documentation/CoreBluetooth/Reference/AppleNotificationCenterServiceSpecification/Specification/Specification.html

```
Due to the nature of iOS, the ANCS is not guaranteed to always be present. As a result, the NC should look for and subscribe to the Service Changed characteristic of the GATT service
```

This makes things complicated, I need to handle it :(

Need to keep advertising according to https://developer.apple.com/forums/thread/665770
Otherwise, iphone will not reconnect automatically

new logo idea, rust crab with iphone
