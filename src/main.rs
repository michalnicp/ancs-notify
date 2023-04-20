use bluer::{
    adv::Advertisement,
    agent::{Agent, ReqError},
    gatt::remote::{Characteristic, CharacteristicWriteRequest, Descriptor},
    Adapter, AdapterEvent, Address, AddressType, Device, Session, Uuid, UuidExt,
};
use futures::{
    future, pin_mut,
    stream::{self, SelectAll},
    FutureExt, StreamExt, TryFutureExt,
};
use rand::Rng;
use std::collections::HashMap;
use std::io::stdin;
use std::thread;
use tokio::{
    pin, select, signal,
    sync::mpsc,
    time::{self, Duration},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// Apple Notification Center Service UUID
const ANCS_UUID: Uuid = Uuid::from_u128(0x7905f431_b5ce_4e99_a40f_4b1e122d00d0);

// Notification Source Characteristic UUID
const ANCS_NOTIFICATION_SOURCE_UUID: Uuid = Uuid::from_u128(0x9fbf120d_6301_42d9_8c58_25e699a21dbd);

// Control Point Characteristic UUID
const ANCS_CONTROL_POINT_UUID: Uuid = Uuid::from_u128(0x69d1d8f3_45e1_49a8_9821_9bbdfdaad9d9);

// Data Source Characteristic UUID
const ANCS_DATA_SOURCE_UUID: Uuid = Uuid::from_u128(0x22eac6e9_24d6_4bb5_be44_b36ace7c7bfb);

// GATT Client Configuration Characteristic UUID
// const GATT_CHARACTERISTIC_DESCRIPTOR_CLIENT_CONFIGURATION: Uuid = Uuid::from_u16(0x2902);
const GATT_CCC_UUID: Uuid = Uuid::from_u128(0x00002902_0000_1000_8000_00805f9b34fb);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    // Subscribe to notifications
    // let data_source_notifications = data_source.notify().await?;
    // tokio::spawn(async move {
    //     pin!(data_source_notifications);
    //     loop {
    //         if let Some(data) = data_source_notifications.next().await {
    //             println!("data source notification: {:x?}", data);
    //         }
    //     }
    // });
    //
    // let notification_source_notifications = notification_source.notify().await?;
    // pin!(notification_source_notifications);
    // while let Some(data) = notification_source_notifications.next().await {
    //     let notification = Notification::parse(&data)?;
    //     println!("notification source notification: {:x?}", data);
    // }

    // TODO: Register authentication callbacks
    // Register custom agent to handle the authentication
    let session1 = session.clone();
    let session2 = session.clone();
    let agent = Agent {
        request_default: true, // TODO: should this be true?
        request_pin_code: Some(Box::new(|req| {
            Box::pin(async move {
                let mut rng = rand::thread_rng();
                let pin_code = rng.gen_range(0..999999);
                // println!("Generated PIN code \"{}\"", pin_code);
                Ok(format!("{:06}", pin_code))
            })
        })),
        display_pin_code: Some(Box::new(|req| {
            Box::pin(async move {
                println!(
                    "PIN code for device {} on {} is \"{}\"",
                    &req.device, &req.adapter, req.pincode
                );
                Ok(())
            })
        })),
        request_passkey: Some(Box::new(move |req| {
            Box::pin(async move {
                let mut rng = rand::thread_rng();
                let passkey = rng.gen_range(0..999999);
                // println!("Generated passkey \"{:06}\"", passkey);
                Ok(passkey)
            })
        })),
        display_passkey: Some(Box::new(|req| {
            Box::pin(async move {
                println!(
                    "Passkey for device {} on {} is \"{:06}\"",
                    &req.device, &req.adapter, req.passkey
                );
                Ok(())
            })
        })),
        request_confirmation: Some(Box::new(move |req| {
            let session1 = session1.clone();
            Box::pin(async move {
                let msg = format!(
                    "Is passkey \"{:06}\" correct for device {} on {}? (y/n)",
                    req.passkey, &req.device, &req.adapter
                );
                if !prompt(msg).await {
                    return Err(ReqError::Rejected);
                }

                // TODO: check passkey
                // println!("Trusting device (request confirmation) {}", &req.device);
                let adapter = session1.adapter(&req.adapter).unwrap();
                let device = adapter.device(req.device).unwrap();
                if let Err(err) = device.set_trusted(true).await {
                    println!("Cannot trust device: {}", &err);
                }
                Ok(())
            })
        })),
        request_authorization: Some(Box::new(move |req| {
            let session2 = session2.clone();
            Box::pin(async move {
                // TODO: check passkey
                // println!("Trusting device (request authorization) {}", &req.device);
                let adapter = session2.adapter(&req.adapter).unwrap();
                let device = adapter.device(req.device).unwrap();
                if let Err(err) = device.set_trusted(true).await {
                    println!("Cannot trust device: {}", &err);
                }
                Ok(())
            })
        })),
        ..Default::default()
    };

    // TODO: unregister agent?
    let _agent_handle = session.register_agent(agent).await?;

    // TODO: only advertise when pairing
    println!(
        "Advertising on Bluetooth adapter {} with address {}",
        adapter.name(),
        adapter.address().await?
    );

    // path=/org/bluez/bluer/advertising/f28ce93a5b914830b2c9d13a1cfc1698
    let le_advertisement = Advertisement {
        advertisement_type: bluer::adv::Type::Peripheral,
        solicit_uuids: vec![ANCS_UUID].into_iter().collect(),
        discoverable: Some(true),
        ..Default::default()
    };
    println!("{:?}", &le_advertisement);
    let handle = adapter.advertise(le_advertisement).await?;

    // handle signals
    println!("Press ctrl-c to quit");

    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // TODO: advertisement is not being removed properly
    println!("Removing advertisement");
    drop(handle);
    time::sleep(Duration::from_secs(3)).await;

    Ok(())
}

async fn read_line() -> String {
    let (tx, mut rx) = mpsc::unbounded_channel();
    thread::spawn(move || {
        let mut buffer = String::new();
        stdin()
            .read_line(&mut buffer)
            .expect("Failed to read line.");
        tx.send(buffer.trim().to_string()).unwrap();
    });
    rx.recv().await.unwrap()
}

async fn prompt(msg: String) -> bool {
    println!("{}", msg);

    // TODO: replace with promptly https://docs.rs/promptly/latest/promptly/
    loop {
        let line = read_line().await;
        if line == "y" {
            return true;
        } else if line == "n" {
            return false;
        } else {
            println!("Please input either y/n");
        }
    }
}

async fn find_device(adapter: &Adapter) -> Result<Option<Device>> {
    for address in adapter.device_addresses().await? {
        let device = adapter.device(address)?;
        let uuids = device.uuids().await?.unwrap_or_default();
        if uuids.contains(&ANCS_UUID) {
            return Ok(Some(device));
        }
    }
    Ok(None)
}

async fn connect(device: &Device) -> Result<()> {
    if !device.is_connected().await? {
        let mut retries = 2;
        loop {
            match device.connect().and_then(|_| device.services()).await {
                Ok(_) => break,
                Err(_) if retries > 0 => {
                    retries -= 1;
                }
                Err(err) => return Err(err.into()),
            }
        }
    } else {
        println!("Already connected to device");
    }
    Ok(())
}

struct AncsClient {
    session: Session,
    adapter: Adapter,
    device: Option<Device>,
    control_point: Option<Characteristic>,
    notification_source: Option<Characteristic>,
    notification_source_ccc: Option<Descriptor>,
    data_source: Option<Characteristic>,
    data_source_ccc: Option<Descriptor>,
    notifications: HashMap<u32, Notification>,
    app_attributes: HashMap<u8
}

impl AncsClient {
    async fn new() -> Result<Self> {
        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;

        Ok(AncsClient {
            session,
            adapter,
            device: None,
            control_point: None,
            notification_source: None,
            notification_source_ccc: None,
            data_source: None,
            data_source_ccc: None,
            notifications: HashMap::new(),
        })
    }

    async fn run(&mut self) -> Result<()> {
        // do all the things
        // can spawn as a task https://github.com/diwic/dbus-rs/blob/ed35a1202606c5be4f7b1a23cfcd9f2e57d3a3f6/dbus-tokio/examples/tokio02_client.rs#L15
        // tokio::spawn(async move { client.run().await });
        self.find_device().await?;
        self.connect().await?;
        self.find_characteristics().await?;

        let notification_stream = self.notification_source.as_ref().unwrap().notify().await?;
        pin!(notification_stream);

        loop {
            select! {
                notification = notification_stream.next() => {
                    // TODO: move to function called handle notification?
                    if let Some(bytes) = notification {
                        if let Ok(notification) = Notification::from_bytes(&bytes) {
                            match notification.event_id {
                                EventID::NotificationAdded => {
                                    self.notifications.insert(notification.notification_uid, notification);

                                    // send command to get notification attributes
                                    // send command to get app attributes
                                    // self.get_app_attributes();
                                },
                                EventID::NotificationModified => {
                                    self.notifications.insert(notification.notification_uid, notification);
                                },
                                EventID::NotificationRemoved => {
                                    self.notifications.remove(&notification.notification_uid);
                                },

                            }
                            // let command = GetNotificationAttributesCommand {
                            //     command_id: notification.command_id,
                            //
                            // };
                            // let command = GetAppAttributesCommand {
                            //     command_id: CommandID::GetAppAttributesCommand,
                            //     app_identifier: notification.
                            // };
                        } else {
                            println!("Failed to parse notification: {:x?}", bytes);
                        }
                    } else {
                        println!("Notification session was terminated");
                    }
                }
            }
        }
    }

    async fn find_device(&mut self) -> Result<()> {
        for address in self.adapter.device_addresses().await? {
            let device = self.adapter.device(address)?;
            let uuids = device.uuids().await?.unwrap_or_default();
            if uuids.contains(&ANCS_UUID) {
                self.device = Some(device);
                return Ok(());
            }
        }
        Err("device not found".into())
    }

    async fn connect(&self) -> Result<()> {
        connect(self.device.as_ref().unwrap()).await
    }

    async fn find_characteristics(&mut self) -> Result<()> {
        self.connect().await?;

        for service in self.device.as_ref().unwrap().services().await? {
            let service_uuid = service.uuid().await?;
            if service_uuid == ANCS_UUID {
                for characteristic in service.characteristics().await? {
                    let characteristic_uuid = characteristic.uuid().await?;
                    match characteristic_uuid {
                        ANCS_NOTIFICATION_SOURCE_UUID => {
                            for descriptor in characteristic.descriptors().await? {
                                let descriptor_uuid = descriptor.uuid().await?;
                                if descriptor_uuid == GATT_CCC_UUID {
                                    self.notification_source_ccc = Some(descriptor);
                                }
                            }
                            self.notification_source = Some(characteristic);
                        }
                        ANCS_CONTROL_POINT_UUID => self.control_point = Some(characteristic),
                        ANCS_DATA_SOURCE_UUID => {
                            for descriptor in characteristic.descriptors().await? {
                                let descriptor_uuid = descriptor.uuid().await?;
                                if descriptor_uuid == GATT_CCC_UUID {
                                    self.data_source_ccc = Some(descriptor);
                                }
                            }
                            self.data_source = Some(characteristic)
                        }
                        _ => {}
                    }
                }
            }
        }

        if self.notification_source.is_none() {
            return Err("notification source not found".into());
        }
        if self.notification_source_ccc.is_none() {
            return Err("notification source ccc not found".into());
        }
        if self.control_point.is_none() {
            return Err("control point not found".into());
        }
        if self.data_source.is_none() {
            return Err("data source not found".into());
        }
        if self.data_source_ccc.is_none() {
            return Err("data source ccc not found".into());
        }

        println!("All ANCS charactertistics found");

        Ok(())
    }

    //  handle_notification_source_notification
    async fn handle_notification(notif: &Notification) -> Result<()> {
        // Get application id and name

        Ok(())
    }

    async fn get_app_attributes(self, app_identifier: &str) -> Result<()> {
        // write command to control characteristic
        let command = GetAppAttributesCommand {
            command_id: CommandID::GetAppAttributes,
            app_identifier: app_identifier.into(),
            attribute_ids: vec![AppAttributeID::DisplayName],
        };

        self.control_point.as_ref().unwrap().write(&command.to_bytes()).await?;

        Ok(())
    }
}

#[derive(Debug)]
struct Notification {
    event_id: EventID,
    event_flags: u8,
    category_id: u8,
    category_count: u8,
    notification_uid: u32,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
enum EventID {
    NotificationAdded,
    NotificationModified,
    NotificationRemoved,
}

impl EventID {
    fn from_u8(n: u8) -> Option<Self> {
        // TODO: use num_enum
        // https://github.com/illicitonion/num_enum/issues/61#issuecomment-955804109
        // if n < Attribute::Invalid as u8 {
        // if n >= Attribute::AppIdentifier as u8 && n <= Attribute::NegativeActionLabel as u8 {
        if n > EventID::NotificationRemoved as u8 {
            None
        } else {
            Some(unsafe { std::mem::transmute::<u8, EventID>(n) })
        }
    }
}

impl Notification {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 8 {
            return Err("invalid length".into());
        }
        Ok(Notification {
            event_id: EventID::from_u8(bytes[0]).unwrap(),
            event_flags: u8::from_le_bytes([bytes[1]]),
            category_id: u8::from_le_bytes([bytes[2]]),
            category_count: u8::from_le_bytes([bytes[3]]),
            notification_uid: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        })
    }
}

#[repr(u8)]
enum NotificationAttributeID {
    AppIdentifier,
    Title,    // followed by 2-byte length
    Subtitle, // followed by 2-byte length
    Message,  // followed by 2-byte length
    MessageSize,
    Date, // UTC#35 yyyyMMdd'T'HHmmSS
    PositiveActionLabel,
    NegativeActionLabel,
}

impl NotificationAttributeID {
    fn from_u8(n: u8) -> Option<Self> {
        // TODO: use num_enum
        // https://github.com/illicitonion/num_enum/issues/61#issuecomment-955804109
        // if n < Attribute::Invalid as u8 {
        // if n >= NotificationAttributeID::AppIdentifier as u8 && n <= NotificationAttributeID::NegativeActionLabel as u8 {
        if n <= NotificationAttributeID::NegativeActionLabel as u8 {
            Some(unsafe { ::std::mem::transmute::<u8, NotificationAttributeID>(n) })
        } else {
            None
        }
    }
}

#[repr(u8)]
enum AppAttributeID {
    DisplayName
}

impl AppAttributeID {
    fn from_u8(n: u8) -> Option<Self> {
        if n <= AppAttributeID::DisplayName as u8 {
            Some(unsafe { ::std::mem::transmute::<u8, AppAttributeID>(n) })
        } else {
            None
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
enum CommandID {
    GetNotificationAttributes,
    GetAppAttributes,
    PerformNotificationAction,
}

struct GetNotificationAttributesCommand {
    command_id: CommandID, // should be set to  GetNotificationAttributes
    notification_id: u32,
    attribute_ids: Vec<(NotificationAttributeID, u16)>, // length is optional
}

impl GetNotificationAttributesCommand {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend([self.command_id as u8]);
        bytes.extend(self.notification_id.to_le_bytes());
        for (attribute_id, length) in self.attribute_ids.iter() {
            bytes.extend([*attribute_id as u8]);
            bytes.extend(length.to_le_bytes());
        }
        bytes
    }
}

struct GetAppAttributesCommand {
    command_id: CommandID,  // should be set to GetAppAttributes
    app_identifier: String, // null terminated
    attribute_ids: Vec<AppAttributeID>,
}

impl GetAppAttributesCommand {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend([self.command_id as u8]);
        bytes.extend(self.app_identifier.as_bytes());
        bytes.extend(self.attribute_ids.iter().map(|x| *x as u8).collect::<Vec<_>>());
        bytes
    }
}

struct PerformNotificationActionCommand {
    command_id: CommandID, // should be set to PerformNotification
    notification_uid: u32,
    // action_id: u8,
}
