use std::fmt;

use serde::Deserialize;
use serde_json::value::RawValue;

use super::TransportError;

/// A REST call's result
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A failed REST call
#[derive(Debug)]
pub enum Error {
    /// No response was received
    Transport(TransportError),

    /// Discord answered with a non-2xx status other than 401 and 429
    Api(ApiError),

    /// Rate limited for longer than the client is allowed to wait, or still limited after retrying
    RateLimited(RateLimited),

    /// 401: the bot token is missing, invalid, or the interaction token expired
    Unauthorized(Option<ApiError>),

    /// The request body could not be encoded
    Encode(serde_json::Error),

    /// The response body did not match the expected type
    Decode(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Transport(e) => write!(f, "request failed: {e}"),
            Error::Api(e) => write!(f, "Discord API error: {e}"),
            Error::RateLimited(e) => write!(f, "rate limited, retry after {}s", e.retry_after),
            Error::Unauthorized(Some(e)) => write!(f, "unauthorized: {e}"),
            Error::Unauthorized(None) => f.write_str("unauthorized"),
            Error::Encode(e) => write!(f, "could not encode request: {e}"),
            Error::Decode(e) => write!(f, "unexpected response from Discord: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Transport(e) => Some(e),
            Error::Encode(e) | Error::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransportError> for Error {
    fn from(error: TransportError) -> Self {
        Error::Transport(error)
    }
}

impl Error {
    /// Discord's JSON error code, if Discord sent one
    pub fn code(&self) -> Option<JsonErrorCode> {
        match self {
            Error::Api(e) | Error::Unauthorized(Some(e)) => Some(e.code),
            _ => None,
        }
    }
}

/// [Error response](https://discord.com/developers/docs/reference#error-messages) body plus status
#[derive(Debug)]
pub struct ApiError {
    /// The HTTP status
    pub status: u16,
    /// Discord's error code
    pub code: JsonErrorCode,
    /// Discord's error message
    pub message: String,
    /// Per-field validation errors, as sent
    pub errors: Option<Box<RawValue>>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} ({})",
            self.status,
            self.message,
            self.code.to_u32()
        )?;
        if let Some(errors) = &self.errors {
            write!(f, ": {}", errors.get())?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct ApiErrorBody {
    #[serde(default)]
    code: u32,
    #[serde(default)]
    message: String,
    errors: Option<Box<RawValue>>,
}

impl ApiError {
    /// `None` when the body is not Discord's error shape (e.g. a Cloudflare HTML page)
    pub(crate) fn parse(status: u16, body: &[u8]) -> Option<Self> {
        let body: ApiErrorBody = serde_json::from_slice(body).ok()?;
        Some(ApiError {
            status,
            code: JsonErrorCode::from_u32(body.code),
            message: body.message,
            errors: body.errors,
        })
    }

    pub(crate) fn unparsed(status: u16, body: &[u8]) -> Self {
        ApiError {
            status,
            code: JsonErrorCode::Other(0),
            message: String::from_utf8_lossy(body).into_owned(),
            errors: None,
        }
    }
}

/// A 429 the client did not (or could no longer) wait out
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimited {
    /// Seconds until the request may be retried
    pub retry_after: f64,
    /// Whether the global (per-bot) limit was hit rather than a route's
    pub global: bool,
    /// `X-RateLimit-Bucket`, when sent
    pub bucket: Option<String>,
}

macro_rules! json_error_codes {
    ($($(#[$meta:meta])* $name:ident = $code:literal,)*) => {
        /// [JSON error code](https://discord.com/developers/docs/topics/opcodes-and-status-codes#json-json-error-codes).
        /// Open: codes not listed here are [`JsonErrorCode::Other`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        #[allow(missing_docs)] // named after Discord's descriptions
        pub enum JsonErrorCode {
            $($(#[$meta])* $name,)*
            /// A code not listed here
            Other(u32),
        }

        impl JsonErrorCode {
            /// The code numbered `code`
            pub fn from_u32(code: u32) -> Self {
                match code {
                    $($code => JsonErrorCode::$name,)*
                    other => JsonErrorCode::Other(other),
                }
            }

            /// The code's number
            pub fn to_u32(self) -> u32 {
                match self {
                    $(JsonErrorCode::$name => $code,)*
                    JsonErrorCode::Other(code) => code,
                }
            }
        }
    };
}

json_error_codes! {
    General = 0,
    UnknownAccount = 10001,
    UnknownApplication = 10002,
    UnknownChannel = 10003,
    UnknownGuild = 10004,
    UnknownIntegration = 10005,
    UnknownInvite = 10006,
    UnknownMember = 10007,
    UnknownMessage = 10008,
    UnknownPermissionOverwrite = 10009,
    UnknownRole = 10011,
    UnknownToken = 10012,
    UnknownUser = 10013,
    UnknownEmoji = 10014,
    UnknownWebhook = 10015,
    UnknownWebhookService = 10016,
    UnknownBan = 10026,
    UnknownSku = 10027,
    UnknownInteraction = 10062,
    UnknownApplicationCommand = 10063,
    UnknownApplicationCommandPermissions = 10066,
    UnknownStageInstance = 10067,
    UnknownGuildScheduledEvent = 10070,
    UnknownSticker = 10060,
    UnknownEntitlement = 10073,
    BotsCannotUseEndpoint = 20001,
    OnlyBotsCanUseEndpoint = 20002,
    MessageEditedOnAnnouncementRateLimit = 20022,
    ChannelWriteRateLimit = 20028,
    ServerWriteRateLimit = 20029,
    MaximumGuildsReached = 30001,
    MaximumPinsReached = 30003,
    MaximumRolesReached = 30005,
    MaximumWebhooksReached = 30007,
    MaximumReactionsReached = 30010,
    MaximumChannelsReached = 30013,
    MaximumApplicationCommandsReached = 30032,
    MaximumThreadParticipantsReached = 30033,
    MaximumBansReached = 30035,
    Unauthorized = 40001,
    AccountVerificationRequired = 40002,
    RequestEntityTooLarge = 40005,
    FeatureTemporarilyDisabled = 40006,
    UserBannedFromGuild = 40007,
    TargetUserNotConnectedToVoice = 40032,
    MessageAlreadyCrossposted = 40033,
    ApplicationCommandNameExists = 40041,
    InteractionAlreadyAcknowledged = 40060,
    TagNamesMustBeUnique = 40061,
    EntitlementAlreadyGranted = 40074,
    MissingAccess = 50001,
    InvalidAccountType = 50002,
    CannotExecuteOnDmChannel = 50003,
    GuildWidgetDisabled = 50004,
    CannotEditMessageByOtherUser = 50005,
    CannotSendEmptyMessage = 50006,
    CannotSendMessagesToUser = 50007,
    CannotSendMessagesInNonTextChannel = 50008,
    ChannelVerificationLevelTooHigh = 50009,
    MissingPermissions = 50013,
    InvalidAuthenticationToken = 50014,
    NoteTooLong = 50015,
    InvalidBulkDeleteCount = 50016,
    CannotPinMessageInOtherChannel = 50019,
    InvalidOrTakenInviteCode = 50020,
    CannotExecuteOnSystemMessage = 50021,
    CannotExecuteOnChannelType = 50024,
    InvalidOAuth2AccessToken = 50025,
    InvalidWebhookToken = 50027,
    InvalidRole = 50028,
    InvalidRecipients = 50033,
    BulkDeleteMessageTooOld = 50034,
    InvalidFormBody = 50035,
    InviteAcceptedToGuildWithoutBot = 50036,
    InvalidApiVersion = 50041,
    FileTooLarge = 50045,
    InvalidFileUploaded = 50046,
    CannotSelfRedeemGift = 50054,
    PaymentSourceRequired = 50070,
    CannotDeleteRequiredCommunityChannel = 50074,
    InvalidStickerSent = 50081,
    ThreadArchived = 50083,
    InvalidThreadNotificationSettings = 50084,
    BeforeValueEarlierThanThreadCreation = 50085,
    ServerNotAvailableInLocation = 50095,
    ServerNeedsMonetization = 50097,
    CannotModifySystemWebhook = 50600,
    TwoFactorRequired = 60003,
    NoUsersWithDiscordTag = 80004,
    ReactionBlocked = 90001,
    ApplicationNotYetAvailable = 110001,
    ApiResourceOverloaded = 130000,
    StageAlreadyOpen = 150006,
    ThreadAlreadyCreatedForMessage = 160004,
    ThreadLocked = 160005,
    MaximumActiveThreadsReached = 160006,
    MaximumActiveAnnouncementThreadsReached = 160007,
    InvalidLottieJson = 170001,
    StickerMaximumFramerateExceeded = 170006,
    MaximumPollAnswersExceeded = 520001,
    CannotExpireNonPollMessage = 520006,
    AutoModerationBlocked = 200000,
    AutoModerationTitleBlocked = 200001,
    WebhooksCanOnlyCreateThreadsInForums = 220003,
    HarmfulLinkBlocked = 240000,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_error_codes_round_trip() {
        assert_eq!(
            JsonErrorCode::from_u32(40060),
            JsonErrorCode::InteractionAlreadyAcknowledged
        );
        assert_eq!(JsonErrorCode::from_u32(1), JsonErrorCode::Other(1));
        assert_eq!(JsonErrorCode::MissingPermissions.to_u32(), 50013);
    }

    #[test]
    fn parses_discord_error_body() {
        let body =
            br#"{"code":50035,"message":"Invalid Form Body","errors":{"content":{"_errors":[]}}}"#;
        let error = ApiError::parse(400, body).unwrap();
        assert_eq!(error.code, JsonErrorCode::InvalidFormBody);
        assert_eq!(error.message, "Invalid Form Body");
        assert!(error.errors.is_some());
        assert!(ApiError::parse(502, b"<html>").is_none());
    }
}
