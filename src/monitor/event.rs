use super::MonitorMessageError;

/// Convenience trait to be able to use `from_raw` and `to_raw` on any value that either defines it
/// or is a `u32`. It doesn't matter that others don't implement this trait, rustc is smart enough
/// to find that out.
trait U32Ext: Sized {
    fn from_raw(value: u32) -> Option<Self>;

    fn to_raw(self) -> u32;
}

impl U32Ext for u32 {
    fn from_raw(value: u32) -> Option<Self> {
        Some(value)
    }

    fn to_raw(self) -> Self {
        self
    }
}

macro_rules! type_or_u32 {
    ($type:ty) => {
        $type
    };
    () => {
        u32
    };
}

macro_rules! define_handshake_failure_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $enum_name:ident {
            $(
                $name:ident = $zmq_sys_name:ident,
            )*
        }
    ) => {
        $(#[$attr])*
        $vis enum $enum_name {
            $(
                $name = zmq_sys::$zmq_sys_name,
            )*
        }

        impl $enum_name {
            pub fn from_raw(data: u32) -> Option<Self> {
                Some(match data {
                    $(
                        zmq_sys::$zmq_sys_name => Self::$name,
                    )*
                    _ => return None,
                })
            }

            pub fn to_raw(self) -> u32 {
                self as u32
            }
        }
    };
}

define_handshake_failure_enum! {
    /// Possible values for the ZMQ_EVENT_HANDSHAKE_FAILED_PROTOCOL socket event.
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HandshakeFailure {
        ZmtpUnspecified = ZMQ_PROTOCOL_ERROR_ZMTP_UNSPECIFIED,
        ZmtpUnexpectedCommand = ZMQ_PROTOCOL_ERROR_ZMTP_UNEXPECTED_COMMAND,
        ZmtpInvalidSequence = ZMQ_PROTOCOL_ERROR_ZMTP_INVALID_SEQUENCE,
        ZmtpKeyExchange = ZMQ_PROTOCOL_ERROR_ZMTP_KEY_EXCHANGE,
        ZmtpMalformedCommandUnspecified = ZMQ_PROTOCOL_ERROR_ZMTP_MALFORMED_COMMAND_UNSPECIFIED,
        ZmtpMalformedCommandMessage = ZMQ_PROTOCOL_ERROR_ZMTP_MALFORMED_COMMAND_MESSAGE,
        ZmtpMalformedCommandHello = ZMQ_PROTOCOL_ERROR_ZMTP_MALFORMED_COMMAND_HELLO,
        ZmtpMalformedCommandInitiate = ZMQ_PROTOCOL_ERROR_ZMTP_MALFORMED_COMMAND_INITIATE,
        ZmtpMalformedCommandError = ZMQ_PROTOCOL_ERROR_ZMTP_MALFORMED_COMMAND_ERROR,
        ZmtpMalformedCommandReady = ZMQ_PROTOCOL_ERROR_ZMTP_MALFORMED_COMMAND_READY,
        ZmtpMalformedCommandWelcome = ZMQ_PROTOCOL_ERROR_ZMTP_MALFORMED_COMMAND_WELCOME,
        ZmtpInvalidMetadata = ZMQ_PROTOCOL_ERROR_ZMTP_INVALID_METADATA,
        ZmtpCryptographic = ZMQ_PROTOCOL_ERROR_ZMTP_CRYPTOGRAPHIC,
        ZmtpMechanismMismatch = ZMQ_PROTOCOL_ERROR_ZMTP_MECHANISM_MISMATCH,
        ZapUnspecified = ZMQ_PROTOCOL_ERROR_ZAP_UNSPECIFIED,
        ZapMalformedReply = ZMQ_PROTOCOL_ERROR_ZAP_MALFORMED_REPLY,
        ZapBadRequestId = ZMQ_PROTOCOL_ERROR_ZAP_BAD_REQUEST_ID,
        ZapBadVersion = ZMQ_PROTOCOL_ERROR_ZAP_BAD_VERSION,
        ZapInvalidStatusCode = ZMQ_PROTOCOL_ERROR_ZAP_INVALID_STATUS_CODE,
        ZapInvalidMetadata = ZMQ_PROTOCOL_ERROR_ZAP_INVALID_METADATA,
    }
}

macro_rules! define_socket_event_enum {
    (
        $(#[$attr:meta])*
        $vis:vis enum $enum_name:ident {
            $(
                $name:ident $(($value:ident $(: $type:ty)?))? = $zmq_sys_name:ident,
            )*
        }
    ) => {
        $(#[$attr])*
        $vis enum $enum_name {
            $(
                $name $({ $value: type_or_u32!($($type)?) })?,
            )*
            Unknown { event: u16, data: u32 },
        }

        impl $enum_name {
            pub fn from_raw(event: u16, data: u32) -> Option<Self> {
                Some(match event as u32 {
                    $(
                        zmq_sys::$zmq_sys_name => Self::$name $({ $value: <type_or_u32!($($type)?)>::from_raw(data)? })?,
                    )*
                    _ => Self::Unknown { event, data },
                })
            }

            pub fn to_raw(self) -> (u16, Option<u32>) {
                match self {
                    $(
                        Self::$name $({ $value })? => (zmq_sys::$zmq_sys_name as u16, ($(Some($value.to_raw()), )? None::<u32>,).0),
                    )*
                    Self::Unknown { event, data } => (event, Some(data)),
                }
            }
        }
    };
}

define_socket_event_enum! {
    /// An event from one of the connected sockets. See the "SUPPORTED EVENTS" section in the
    /// "zmq_socket_monitor" manual page (`man zmq_socket_monitor`) for the original documentation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SocketEvent {
        Connected(fd) = ZMQ_EVENT_CONNECTED,
        ConnectDelayed = ZMQ_EVENT_CONNECT_DELAYED,
        ConnectRetried(interval) = ZMQ_EVENT_CONNECT_RETRIED,
        Listening(fd) = ZMQ_EVENT_LISTENING,
        BindFailed(errno) = ZMQ_EVENT_BIND_FAILED,
        Accepted(fd) = ZMQ_EVENT_ACCEPTED,
        AcceptFailed(errno) = ZMQ_EVENT_ACCEPT_FAILED,
        Closed(fd) = ZMQ_EVENT_CLOSED,
        CloseFailed(errno) = ZMQ_EVENT_CLOSE_FAILED,
        Disconnected(fd) = ZMQ_EVENT_DISCONNECTED,
        MonitorStopped = ZMQ_EVENT_MONITOR_STOPPED,
        HandshakeFailedNoDetail(fd) = ZMQ_EVENT_HANDSHAKE_FAILED_NO_DETAIL,
        HandshakeSucceeded = ZMQ_EVENT_HANDSHAKE_SUCCEEDED,
        HandshakeFailedProtocol(err: HandshakeFailure) = ZMQ_EVENT_HANDSHAKE_FAILED_PROTOCOL,
        HandshakeFailedAuth(error_code) = ZMQ_EVENT_HANDSHAKE_FAILED_AUTH,
    }
}

impl SocketEvent {
    pub fn parse_from(msg: &zmq::Message) -> Result<Self, MonitorMessageError> {
        let bytes = &**msg;

        let event: [u8; 6] = bytes
            .try_into()
            .map_err(|_| MonitorMessageError::InvalidEventFrameLength(bytes.len()))?;
        let event_type = u16::from_ne_bytes(event[0..2].try_into().unwrap());
        let data = u32::from_ne_bytes(event[2..6].try_into().unwrap());

        SocketEvent::from_raw(event_type, data)
            .ok_or(MonitorMessageError::InvalidEventData(event_type, data))
    }
}
