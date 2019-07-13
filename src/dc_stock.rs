/// Stock strings
///
/// These identify the string to return in [Context.stock_str].  The
/// numbers must stay in sync with `deltachat.h` `DC_STR_*` constants.
///
/// See the `stock_*` methods on [Context] to use these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum StockId {
    NoMessages = 1,
    SelfMsg = 2,
    Draft = 3,
    Member = 4,
    Contact = 6,
    VoiceMessage = 7,
    DeadDrop = 8,
    Image = 9,
    Video = 10,
    Audio = 11,
    File = 12,
    StatusLine = 13,
    NewGroupDraft = 14,
    MsgGrpName = 15,
    MsgGrpImgChanged = 16,
    MsgAddMember = 17,
    MsgDelMember = 18,
    MsgGroupLeft = 19,
    Gif = 23,
    EncryptedMsg = 24,
    E2E_Available = 25,
    Encr_Transp = 27,
    Encr_None = 28,
    CantDecrypt_Msg_Body = 29,
    FingerPrints = 30,
    ReadRcpt = 31,
    ReadRcpt_MailBody = 32,
    MsgGrpImgDeleted = 33,
    E2E_Preferred = 34,
    Contact_Verified = 35,
    Contact_Not_Verified = 36,
    Contact_Setup_Changed = 37,
    ArchivedChats = 40,
    StarredMsgs = 41,
    AC_Setup_Msg_Subject = 42,
    AC_Setup_Msg_Body = 43,
    SelfTalk_SubTitle = 50,
    Cannot_Login = 60,
    Server_Response = 61,
    MsgActionByUser = 62,
    MsgActionByMe = 63,
    MsgLocationEnabled = 64,
    MsgLocationDisabled = 65,
    Location = 66,
}

/// Default untranslated strings for stock messages.
///
/// These could be used in logging calls, so no logging here.
pub fn default_string(id: StockId) -> String {
    match id {
        StockId::NoMessages => String::from("No messages."),
        StockId::SelfMsg => String::from("Me"),
        StockId::Draft => String::from("Draft"),
        StockId::Member => String::from("%1$s member(s)"),
        StockId::Contact => String::from("%1$s contact(s)"),
        StockId::VoiceMessage => String::from("Voice message"),
        StockId::DeadDrop => String::from("Contact requests"),
        StockId::Image => String::from("Image"),
        StockId::Gif => String::from("GIF"),
        StockId::Video => String::from("Video"),
        StockId::Audio => String::from("Audio"),
        StockId::File => String::from("File"),
        StockId::Location => String::from("Location"),
        StockId::EncryptedMsg => String::from("Encrypted message"),
        StockId::StatusLine =>
            String::from("Sent with my Delta Chat Messenger: https://delta.chat"),
        StockId::NewGroupDraft =>
            String::from("Hello, I\'ve just created the group \"%1$s\" for us."),
        StockId::MsgGrpName =>
            String::from("Group name changed from \"%1$s\" to \"%2$s\"."),
        StockId::MsgGrpImgChanged => String::from("Group image changed."),
        StockId::MsgAddMember => String::from("Member %1$s added."),
        StockId::MsgDelMember => String::from("Member %1$s removed."),
        StockId::MsgGroupLeft => String::from("Group left."),
        StockId::MsgLocationEnabled => String::from("Location streaming enabled."),
        StockId::MsgLocationDisabled => String::from("Location streaming disabled."),
        StockId::MsgActionByUser => String::from("%1$s by %2$s."),
        StockId::MsgActionByMe => String::from("%1$s by me."),
        StockId::E2E_Available => String::from("End-to-end encryption available."),
        StockId::Encr_Transp => String::from("Transport-encryption."),
        StockId::Encr_None => String::from("No encryption."),
        StockId::FingerPrints => String::from("Fingerprints"),
        StockId::ReadRcpt => String::from("Return receipt"),
        StockId::ReadRcpt_MailBody =>
            String::from("This is a return receipt for the message \"%1$s\"."),
        StockId::MsgGrpImgDeleted => String::from("Group image deleted."),
        StockId::E2E_Preferred => String::from("End-to-end encryption preferred."),
        StockId::Contact_Verified => String::from("%1$s verified."),
        StockId::Contact_Not_Verified => String::from("Cannot verify %1$s"),
        StockId::Contact_Setup_Changed => String::from("Changed setup for %1$s"),
        StockId::ArchivedChats => String::from("Archived chats"),
        StockId::StarredMsgs => String::from("Starred messages"),
        StockId::AC_Setup_Msg_Subject => String::from("Autocrypt Setup Message"),
        StockId::AC_Setup_Msg_Body =>
            String::from("This is the Autocrypt Setup Message used to transfer your key between clients.\n\nTo decrypt and use your key, open the message in an Autocrypt-compliant client and enter the setup code presented on the generating device."),
        StockId::SelfTalk_SubTitle => String::from("Messages I sent to myself"),
        StockId::CantDecrypt_Msg_Body =>
            String::from("This message was encrypted for another setup."),
        StockId::Cannot_Login => String::from("Cannot login as %1$s."),
        StockId::Server_Response => String::from("Response from %1$s: %2$s"),
    }
}
