# ancs-notify

Bluetooth app (linux only) for connecting as a peripheral to an iOS device to
receive notifications over ANCS. The primary use is to display desktop notifications.

## TODO

- [x] Advertise and pair with central
- [x] Receive notifications
- [ ] Fetch app attributes and notification attributes
- [ ] Cache notifications and attributes
- [ ] Print notifications to stdout
- [ ] Support auto reconnect, non-default adapter
- [ ] Custom notification handler (other than D-Bus) with additional features tbd
- [ ] Package and release (service, config, AUR, etc)

## Debug BlueZ using D-Bus

```
dbus-monitor --system "type='signal',sender='org.bluez'"
```
