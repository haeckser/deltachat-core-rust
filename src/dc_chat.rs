use crate::constants::*;
use crate::context::Context;
use crate::dc_array::*;
use crate::dc_chatlist::*;
use crate::dc_contact::*;
use crate::dc_job::*;
use crate::dc_log::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::StockId;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

/**
 * @class dc_chat_t
 *
 * An object representing a single chat in memory.
 * Chat objects are created using eg. dc_get_chat()
 * and are not updated on database changes;
 * if you want an update, you have to recreate the object.
 */
#[derive(Copy, Clone)]
pub struct Chat<'a> {
    magic: uint32_t,
    pub id: uint32_t,
    pub type_0: libc::c_int,
    pub name: *mut libc::c_char,
    archived: libc::c_int,
    pub context: &'a Context,
    pub grpid: *mut libc::c_char,
    blocked: libc::c_int,
    pub param: *mut dc_param_t,
    pub gossiped_timestamp: i64,
    is_sending_locations: libc::c_int,
}

// handle chats
pub unsafe fn dc_create_chat_by_msg_id(context: &Context, msg_id: uint32_t) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut send_event: libc::c_int = 0i32;
    let msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let chat: *mut Chat = dc_chat_new(context);
    if dc_msg_load_from_db(msg, context, msg_id)
        && dc_chat_load_from_db(chat, (*msg).chat_id)
        && (*chat).id > 9i32 as libc::c_uint
    {
        chat_id = (*chat).id;
        if 0 != (*chat).blocked {
            dc_unblock_chat(context, (*chat).id);
            send_event = 1i32
        }
        dc_scaleup_contact_origin(context, (*msg).from_id, 0x800i32);
    }

    dc_msg_unref(msg);
    dc_chat_unref(chat);
    if 0 != send_event {
        context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
    }
    chat_id
}

pub unsafe fn dc_chat_new<'a>(context: &'a Context) -> *mut Chat<'a> {
    let mut chat: *mut Chat;
    chat = calloc(1, ::std::mem::size_of::<Chat>()) as *mut Chat;
    (*chat).magic = 0xc4a7c4a7u32;
    (*chat).context = context;
    (*chat).type_0 = 0i32;
    (*chat).param = dc_param_new();
    chat
}

pub unsafe fn dc_chat_unref(mut chat: *mut Chat) {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return;
    }
    dc_chat_empty(chat);
    dc_param_unref((*chat).param);
    (*chat).magic = 0i32 as uint32_t;
    free(chat as *mut libc::c_void);
}

pub unsafe fn dc_chat_empty(mut chat: *mut Chat) {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return;
    }
    free((*chat).name as *mut libc::c_void);
    (*chat).name = 0 as *mut libc::c_char;
    (*chat).type_0 = 0i32;
    (*chat).id = 0i32 as uint32_t;
    free((*chat).grpid as *mut libc::c_void);
    (*chat).grpid = 0 as *mut libc::c_char;
    (*chat).blocked = 0i32;
    (*chat).gossiped_timestamp = 0;
    dc_param_set_packed((*chat).param, 0 as *const libc::c_char);
}

pub unsafe fn dc_unblock_chat(context: &Context, chat_id: uint32_t) {
    dc_block_chat(context, chat_id, 0i32);
}

pub unsafe fn dc_block_chat(context: &Context, chat_id: uint32_t, new_blocking: libc::c_int) {
    let stmt: *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"UPDATE chats SET blocked=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, new_blocking);
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

pub unsafe fn dc_chat_load_from_db(chat: *mut Chat, chat_id: uint32_t) -> bool {
    let mut success = false;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(chat.is_null() || (*chat).magic != 0xc4a7c4a7u32) {
        dc_chat_empty(chat);
        stmt = dc_sqlite3_prepare(
            (*chat).context,
            &(*chat).context.sql,
            b"SELECT c.id,c.type,c.name, c.grpid,c.param,c.archived, \
                  c.blocked, c.gossiped_timestamp, c.locations_send_until  \
                  FROM chats c WHERE c.id=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1, chat_id as libc::c_int);

        if !(sqlite3_step(stmt) != 100i32) {
            if !(0 == set_from_stmt(chat, stmt)) {
                success = true
            }
        }
    }
    sqlite3_finalize(stmt);
    success
}

unsafe fn set_from_stmt(mut chat: *mut Chat, row: *mut sqlite3_stmt) -> libc::c_int {
    let mut row_offset: libc::c_int = 0i32;
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 || row.is_null() {
        return 0i32;
    }
    dc_chat_empty(chat);
    let fresh0 = row_offset;
    row_offset = row_offset + 1;
    (*chat).id = sqlite3_column_int(row, fresh0) as uint32_t;
    let fresh1 = row_offset;
    row_offset = row_offset + 1;
    (*chat).type_0 = sqlite3_column_int(row, fresh1);
    let fresh2 = row_offset;
    row_offset = row_offset + 1;
    (*chat).name = dc_strdup(sqlite3_column_text(row, fresh2) as *mut libc::c_char);
    let fresh3 = row_offset;
    row_offset = row_offset + 1;
    (*chat).grpid = dc_strdup(sqlite3_column_text(row, fresh3) as *mut libc::c_char);
    let fresh4 = row_offset;
    row_offset = row_offset + 1;
    dc_param_set_packed(
        (*chat).param,
        sqlite3_column_text(row, fresh4) as *mut libc::c_char,
    );
    let fresh5 = row_offset;
    row_offset = row_offset + 1;
    (*chat).archived = sqlite3_column_int(row, fresh5);
    let fresh6 = row_offset;
    row_offset = row_offset + 1;
    (*chat).blocked = sqlite3_column_int(row, fresh6);
    let fresh7 = row_offset;
    row_offset = row_offset + 1;
    (*chat).gossiped_timestamp = sqlite3_column_int64(row, fresh7) as i64;
    let fresh8 = row_offset;
    row_offset = row_offset + 1;
    (*chat).is_sending_locations =
        (sqlite3_column_int64(row, fresh8) as i64 > time()) as libc::c_int;
    if (*chat).id == 1i32 as libc::c_uint {
        free((*chat).name as *mut libc::c_void);
        let cstring = to_cstring((*chat).context.stock_str(StockId::DeadDrop));
        (*chat).name = dc_strdup(cstring.as_ptr());
    } else if (*chat).id == 6i32 as libc::c_uint {
        free((*chat).name as *mut libc::c_void);
        let tempname = to_cstring((*chat).context.stock_str(StockId::ArchivedChats));
        (*chat).name = dc_mprintf(
            b"%s (%i)\x00" as *const u8 as *const libc::c_char,
            tempname.as_ptr(),
            dc_get_archived_cnt((*chat).context),
        );
    } else if (*chat).id == 5i32 as libc::c_uint {
        free((*chat).name as *mut libc::c_void);
        let name = to_cstring((*chat).context.stock_str(StockId::StarredMsgs));
        (*chat).name = dc_strdup(name.as_ptr());
    } else if 0 != dc_param_exists((*chat).param, 'K' as i32) {
        free((*chat).name as *mut libc::c_void);
        let name = to_cstring((*chat).context.stock_str(StockId::SelfMsg));
        (*chat).name = dc_strdup(name.as_ptr());
    }
    row_offset
}

pub unsafe fn dc_create_chat_by_contact_id(context: &Context, contact_id: uint32_t) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_blocked: libc::c_int = 0i32;
    let mut send_event: libc::c_int = 0i32;
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_blocked);
    if 0 != chat_id {
        if 0 != chat_blocked {
            dc_unblock_chat(context, chat_id);
            send_event = 1i32
        }
    } else if !dc_real_contact_exists(context, contact_id) && contact_id != 1i32 as libc::c_uint {
        dc_log_warning(
            context,
            0i32,
            b"Cannot create chat, contact %i does not exist.\x00" as *const u8
                as *const libc::c_char,
            contact_id as libc::c_int,
        );
    } else {
        dc_create_or_lookup_nchat_by_contact_id(
            context,
            contact_id,
            0i32,
            &mut chat_id,
            0 as *mut libc::c_int,
        );
        if 0 != chat_id {
            send_event = 1i32
        }
        dc_scaleup_contact_origin(context, contact_id, 0x800i32);
    }
    if 0 != send_event {
        context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
    }
    chat_id
}

pub unsafe fn dc_create_or_lookup_nchat_by_contact_id(
    context: &Context,
    contact_id: uint32_t,
    create_blocked: libc::c_int,
    ret_chat_id: *mut uint32_t,
    ret_chat_blocked: *mut libc::c_int,
) {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_blocked: libc::c_int = 0i32;
    let contact: *mut dc_contact_t;
    let chat_name: *mut libc::c_char;
    let mut q: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !ret_chat_id.is_null() {
        *ret_chat_id = 0i32 as uint32_t
    }
    if !ret_chat_blocked.is_null() {
        *ret_chat_blocked = 0i32
    }
    if !context.sql.is_open() {
        return;
    }
    if contact_id == 0i32 as libc::c_uint {
        return;
    }
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_blocked);
    if chat_id != 0i32 as libc::c_uint {
        if !ret_chat_id.is_null() {
            *ret_chat_id = chat_id
        }
        if !ret_chat_blocked.is_null() {
            *ret_chat_blocked = chat_blocked
        }
        return;
    }
    contact = dc_contact_new(context);
    if dc_contact_load_from_db(contact, &context.sql, contact_id) {
        chat_name =
            if !(*contact).name.is_null() && 0 != *(*contact).name.offset(0isize) as libc::c_int {
                (*contact).name
            } else {
                (*contact).addr
            };
        q = sqlite3_mprintf(
            b"INSERT INTO chats (type, name, param, blocked, grpid) VALUES(%i, %Q, %Q, %i, %Q)\x00"
                as *const u8 as *const libc::c_char,
            100i32,
            chat_name,
            if contact_id == 1i32 as libc::c_uint {
                b"K=1\x00" as *const u8 as *const libc::c_char
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            create_blocked,
            (*contact).addr,
        );
        stmt = dc_sqlite3_prepare(context, &context.sql, q);
        if !stmt.is_null() {
            if !(sqlite3_step(stmt) != 101i32) {
                chat_id = dc_sqlite3_get_rowid(
                    context,
                    &context.sql,
                    b"chats\x00" as *const u8 as *const libc::c_char,
                    b"grpid\x00" as *const u8 as *const libc::c_char,
                    (*contact).addr,
                );
                sqlite3_free(q as *mut libc::c_void);
                sqlite3_finalize(stmt);
                q = sqlite3_mprintf(
                    b"INSERT INTO chats_contacts (chat_id, contact_id) VALUES(%i, %i)\x00"
                        as *const u8 as *const libc::c_char,
                    chat_id,
                    contact_id,
                );
                stmt = dc_sqlite3_prepare(context, &context.sql, q);
                if !(sqlite3_step(stmt) != 101i32) {
                    sqlite3_free(q as *mut libc::c_void);
                    q = 0 as *mut libc::c_char;
                    sqlite3_finalize(stmt);
                    stmt = 0 as *mut sqlite3_stmt
                }
            }
        }
    }
    sqlite3_free(q as *mut libc::c_void);
    sqlite3_finalize(stmt);
    dc_contact_unref(contact);
    if !ret_chat_id.is_null() {
        *ret_chat_id = chat_id
    }
    if !ret_chat_blocked.is_null() {
        *ret_chat_blocked = create_blocked
    };
}

pub unsafe fn dc_lookup_real_nchat_by_contact_id(
    context: &Context,
    contact_id: uint32_t,
    ret_chat_id: *mut uint32_t,
    ret_chat_blocked: *mut libc::c_int,
) {
    /* checks for "real" chats or self-chat */
    let stmt: *mut sqlite3_stmt;
    if !ret_chat_id.is_null() {
        *ret_chat_id = 0i32 as uint32_t
    }
    if !ret_chat_blocked.is_null() {
        *ret_chat_blocked = 0i32
    }
    if !context.sql.is_open() {
        return;
    }
    stmt =
        dc_sqlite3_prepare(
            context,
            &context.sql,
            b"SELECT c.id, c.blocked FROM chats c INNER JOIN chats_contacts j ON c.id=j.chat_id WHERE c.type=100 AND c.id>9 AND j.contact_id=?;\x00"
                               as *const u8 as *const libc::c_char);
    sqlite3_bind_int(stmt, 1i32, contact_id as libc::c_int);
    if sqlite3_step(stmt) == 100i32 {
        if !ret_chat_id.is_null() {
            *ret_chat_id = sqlite3_column_int(stmt, 0i32) as uint32_t
        }
        if !ret_chat_blocked.is_null() {
            *ret_chat_blocked = sqlite3_column_int(stmt, 1i32)
        }
    }
    sqlite3_finalize(stmt);
}

pub unsafe fn dc_get_chat_id_by_contact_id(context: &Context, contact_id: uint32_t) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut chat_id_blocked: libc::c_int = 0i32;
    dc_lookup_real_nchat_by_contact_id(context, contact_id, &mut chat_id, &mut chat_id_blocked);
    return if 0 != chat_id_blocked {
        0i32 as libc::c_uint
    } else {
        chat_id
    };
}

pub unsafe fn dc_prepare_msg<'a>(
    context: &'a Context,
    chat_id: uint32_t,
    mut msg: *mut dc_msg_t<'a>,
) -> uint32_t {
    if msg.is_null() || chat_id <= 9i32 as libc::c_uint {
        return 0i32 as uint32_t;
    }
    (*msg).state = 18i32;
    let msg_id: uint32_t = prepare_msg_common(context, chat_id, msg);
    context.call_cb(
        Event::MSGS_CHANGED,
        (*msg).chat_id as uintptr_t,
        (*msg).id as uintptr_t,
    );
    return msg_id;
}

unsafe fn prepare_msg_common<'a>(
    context: &'a Context,
    chat_id: uint32_t,
    mut msg: *mut dc_msg_t<'a>,
) -> uint32_t {
    let current_block: u64;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut chat: *mut Chat = 0 as *mut Chat;
    (*msg).id = 0i32 as uint32_t;
    (*msg).context = context;
    if (*msg).type_0 == 10i32 {
        current_block = 17281240262373992796;
    } else if (*msg).type_0 == 20i32
        || (*msg).type_0 == 21i32
        || (*msg).type_0 == 40i32
        || (*msg).type_0 == 41i32
        || (*msg).type_0 == 50i32
        || (*msg).type_0 == 60i32
    {
        pathNfilename = dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
        if pathNfilename.is_null() {
            dc_log_error(
                context,
                0i32,
                b"Attachment missing for message of type #%i.\x00" as *const u8
                    as *const libc::c_char,
                (*msg).type_0 as libc::c_int,
            );
            current_block = 2171833246886114521;
        } else if (*msg).state == 18i32 && 0 == dc_is_blobdir_path(context, pathNfilename) {
            dc_log_error(
                context,
                0i32,
                b"Files must be created in the blob-directory.\x00" as *const u8
                    as *const libc::c_char,
            );
            current_block = 2171833246886114521;
        } else if 0 == dc_make_rel_and_copy(context, &mut pathNfilename) {
            current_block = 2171833246886114521;
        } else {
            dc_param_set((*msg).param, 'f' as i32, pathNfilename);
            if (*msg).type_0 == 60i32 || (*msg).type_0 == 20i32 {
                let mut better_type: libc::c_int = 0i32;
                let mut better_mime: *mut libc::c_char = 0 as *mut libc::c_char;
                dc_msg_guess_msgtype_from_suffix(pathNfilename, &mut better_type, &mut better_mime);
                if 0 != better_type {
                    (*msg).type_0 = better_type;
                    dc_param_set((*msg).param, 'm' as i32, better_mime);
                }
                free(better_mime as *mut libc::c_void);
            } else if 0 == dc_param_exists((*msg).param, 'm' as i32) {
                let mut better_mime_0: *mut libc::c_char = 0 as *mut libc::c_char;
                dc_msg_guess_msgtype_from_suffix(
                    pathNfilename,
                    0 as *mut libc::c_int,
                    &mut better_mime_0,
                );
                dc_param_set((*msg).param, 'm' as i32, better_mime_0);
                free(better_mime_0 as *mut libc::c_void);
            }
            dc_log_info(
                context,
                0i32,
                b"Attaching \"%s\" for message type #%i.\x00" as *const u8 as *const libc::c_char,
                pathNfilename,
                (*msg).type_0 as libc::c_int,
            );
            current_block = 17281240262373992796;
        }
    } else {
        dc_log_error(
            context,
            0i32,
            b"Cannot send messages of type #%i.\x00" as *const u8 as *const libc::c_char,
            (*msg).type_0 as libc::c_int,
        );
        current_block = 2171833246886114521;
    }
    match current_block {
        17281240262373992796 => {
            dc_unarchive_chat(context, chat_id);
            chat = dc_chat_new(context);
            if dc_chat_load_from_db(chat, chat_id) {
                if (*msg).state != 18i32 {
                    (*msg).state = 20i32
                }
                (*msg).id =
                    prepare_msg_raw(context, chat, msg, dc_create_smeared_timestamp(context));
                (*msg).chat_id = chat_id
            }
        }
        _ => {}
    }
    /* potential error already logged */
    dc_chat_unref(chat);
    free(pathNfilename as *mut libc::c_void);

    (*msg).id
}

unsafe fn prepare_msg_raw(
    context: &Context,
    chat: *mut Chat,
    msg: *const dc_msg_t,
    timestamp: i64,
) -> uint32_t {
    let mut do_guarantee_e2ee: libc::c_int;
    let e2ee_enabled: libc::c_int;
    let current_block: u64;
    let mut parent_rfc724_mid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut parent_references: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut parent_in_reply_to: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut new_rfc724_mid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut new_references: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut new_in_reply_to: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut msg_id: uint32_t = 0i32 as uint32_t;
    let mut to_id: uint32_t = 0i32 as uint32_t;
    let mut location_id: uint32_t = 0i32 as uint32_t;

    if !((*chat).type_0 == 100i32 || (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32) {
        dc_log_error(
            context,
            0i32,
            b"Cannot send to chat type #%i.\x00" as *const u8 as *const libc::c_char,
            (*chat).type_0,
        );
    } else if ((*chat).type_0 == 120i32 || (*chat).type_0 == 130i32)
        && 0 == dc_is_contact_in_chat(context, (*chat).id, 1i32 as uint32_t)
    {
        dc_log_event(
            context,
            Event::ERROR_SELF_NOT_IN_GROUP,
            0i32,
            b"Cannot send message; self not in group.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        let from: *mut libc::c_char = dc_sqlite3_get_config(
            context,
            &context.sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if from.is_null() {
            dc_log_error(
                context,
                0i32,
                b"Cannot send message, not configured.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            new_rfc724_mid = dc_create_outgoing_rfc724_mid(
                if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
                    (*chat).grpid
                } else {
                    0 as *mut libc::c_char
                },
                from,
            );
            free(from as *mut libc::c_void);
            if (*chat).type_0 == 100i32 {
                stmt = dc_sqlite3_prepare(
                    context,
                    &context.sql,
                    b"SELECT contact_id FROM chats_contacts WHERE chat_id=?;\x00" as *const u8
                        as *const libc::c_char,
                );
                sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
                if sqlite3_step(stmt) != 100i32 {
                    dc_log_error(
                        context,
                        0i32,
                        b"Cannot send message, contact for chat #%i not found.\x00" as *const u8
                            as *const libc::c_char,
                        (*chat).id,
                    );
                    current_block = 10477488590406205504;
                } else {
                    to_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
                    sqlite3_finalize(stmt);
                    stmt = 0 as *mut sqlite3_stmt;
                    current_block = 5689316957504528238;
                }
            } else {
                if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
                    if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 1i32 {
                        dc_param_set((*chat).param, 'U' as i32, 0 as *const libc::c_char);
                        dc_chat_update_param(chat);
                    }
                }
                current_block = 5689316957504528238;
            }
            match current_block {
                10477488590406205504 => {}
                _ => {
                    /* check if we can guarantee E2EE for this message.
                    if we guarantee E2EE, and circumstances change
                    so that E2EE is no longer available at a later point (reset, changed settings),
                    we do not send the message out at all */
                    do_guarantee_e2ee = 0i32;
                    e2ee_enabled = dc_sqlite3_get_config_int(
                        context,
                        &context.sql,
                        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                        1i32,
                    );
                    if 0 != e2ee_enabled && dc_param_get_int((*msg).param, 'u' as i32, 0i32) == 0i32
                    {
                        let mut can_encrypt: libc::c_int = 1i32;
                        let mut all_mutual: libc::c_int = 1i32;
                        stmt =
                            dc_sqlite3_prepare(context, &context.sql,
                                               b"SELECT ps.prefer_encrypted, c.addr FROM chats_contacts cc  LEFT JOIN contacts c ON cc.contact_id=c.id  LEFT JOIN acpeerstates ps ON c.addr=ps.addr  WHERE cc.chat_id=?  AND cc.contact_id>9;\x00"
                                                   as *const u8 as
                                                   *const libc::c_char);
                        sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
                        while sqlite3_step(stmt) == 100i32 {
                            if sqlite3_column_type(stmt, 0i32) == 5i32 {
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"[autocrypt] no peerstate for %s\x00" as *const u8
                                        as *const libc::c_char,
                                    sqlite3_column_text(stmt, 1i32),
                                );
                                can_encrypt = 0i32;
                                all_mutual = 0i32
                            } else {
                                let prefer_encrypted: libc::c_int = sqlite3_column_int(stmt, 0i32);
                                if prefer_encrypted != 1i32 {
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"[autocrypt] peerstate for %s is %s\x00" as *const u8
                                            as *const libc::c_char,
                                        sqlite3_column_text(stmt, 1i32),
                                        if prefer_encrypted == 0i32 {
                                            b"NOPREFERENCE\x00" as *const u8 as *const libc::c_char
                                        } else {
                                            b"RESET\x00" as *const u8 as *const libc::c_char
                                        },
                                    );
                                    all_mutual = 0i32
                                }
                            }
                        }
                        sqlite3_finalize(stmt);
                        if 0 != can_encrypt {
                            if 0 != all_mutual {
                                do_guarantee_e2ee = 1i32
                            } else if 0
                                != last_msg_in_chat_encrypted(context, &context.sql, (*chat).id)
                            {
                                do_guarantee_e2ee = 1i32
                            }
                        }
                    }
                    if 0 != do_guarantee_e2ee {
                        dc_param_set_int((*msg).param, 'c' as i32, 1i32);
                    }
                    dc_param_set((*msg).param, 'e' as i32, 0 as *const libc::c_char);
                    if 0 == dc_chat_is_self_talk(chat)
                        && 0 != get_parent_mime_headers(
                            chat,
                            &mut parent_rfc724_mid,
                            &mut parent_in_reply_to,
                            &mut parent_references,
                        )
                    {
                        if !parent_rfc724_mid.is_null()
                            && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                        {
                            new_in_reply_to = dc_strdup(parent_rfc724_mid)
                        }
                        if !parent_references.is_null() {
                            let space: *mut libc::c_char;
                            space = strchr(parent_references, ' ' as i32);
                            if !space.is_null() {
                                *space = 0i32 as libc::c_char
                            }
                        }
                        if !parent_references.is_null()
                            && 0 != *parent_references.offset(0isize) as libc::c_int
                            && !parent_rfc724_mid.is_null()
                            && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                        {
                            new_references = dc_mprintf(
                                b"%s %s\x00" as *const u8 as *const libc::c_char,
                                parent_references,
                                parent_rfc724_mid,
                            )
                        } else if !parent_references.is_null()
                            && 0 != *parent_references.offset(0isize) as libc::c_int
                        {
                            new_references = dc_strdup(parent_references)
                        } else if !parent_in_reply_to.is_null()
                            && 0 != *parent_in_reply_to.offset(0isize) as libc::c_int
                            && !parent_rfc724_mid.is_null()
                            && 0 != *parent_rfc724_mid.offset(0isize) as libc::c_int
                        {
                            new_references = dc_mprintf(
                                b"%s %s\x00" as *const u8 as *const libc::c_char,
                                parent_in_reply_to,
                                parent_rfc724_mid,
                            )
                        } else if !parent_in_reply_to.is_null()
                            && 0 != *parent_in_reply_to.offset(0isize) as libc::c_int
                        {
                            new_references = dc_strdup(parent_in_reply_to)
                        }
                    }

                    // add independent location to database

                    if 0 != dc_param_exists((*msg).param, DC_PARAM_SET_LATITUDE as libc::c_int) {
                        stmt = dc_sqlite3_prepare(
                            context,
                            &context.sql,
                            b"INSERT INTO locations \
			      (timestamp,from_id,chat_id, latitude,longitude,independent)\
                              VALUES (?,?,?, ?,?,1);\x00" as *const u8
                                as *const libc::c_char,
                        );
                        sqlite3_bind_int64(stmt, 1, timestamp as i64);
                        sqlite3_bind_int(stmt, 2, DC_CONTACT_ID_SELF as libc::c_int);
                        sqlite3_bind_int(stmt, 3, (*chat).id as libc::c_int);
                        sqlite3_bind_double(
                            stmt,
                            4,
                            dc_param_get_float(
                                (*msg).param,
                                DC_PARAM_SET_LATITUDE as libc::c_int,
                                0.0,
                            ),
                        );
                        sqlite3_bind_double(
                            stmt,
                            5,
                            dc_param_get_float(
                                (*msg).param,
                                DC_PARAM_SET_LONGITUDE as libc::c_int,
                                0.0,
                            ),
                        );
                        sqlite3_step(stmt);
                        sqlite3_finalize(stmt);

                        location_id = dc_sqlite3_get_rowid2(
                            context,
                            &context.sql,
                            b"locations\x00" as *const u8 as *const libc::c_char,
                            b"timestamp\x00" as *const u8 as *const libc::c_char,
                            timestamp as u64,
                            b"from_id\x00" as *const u8 as *const libc::c_char,
                            DC_CONTACT_ID_SELF as u32,
                        );
                    }

                    // add message to the database

                    stmt =
                        dc_sqlite3_prepare(
                            context,&context.sql,
                                           b"INSERT INTO msgs (rfc724_mid, chat_id, from_id, to_id, timestamp, type, state, txt, param, hidden, mime_in_reply_to, mime_references, location_id) VALUES (?,?,?,?,?, ?,?,?,?,?, ?,?,?);\x00"
                                               as *const u8 as
                                               *const libc::c_char);
                    sqlite3_bind_text(stmt, 1i32, new_rfc724_mid, -1i32, None);
                    sqlite3_bind_int(stmt, 2i32, (*chat).id as libc::c_int);
                    sqlite3_bind_int(stmt, 3i32, 1i32);
                    sqlite3_bind_int(stmt, 4i32, to_id as libc::c_int);
                    sqlite3_bind_int64(stmt, 5i32, timestamp as sqlite3_int64);
                    sqlite3_bind_int(stmt, 6i32, (*msg).type_0);
                    sqlite3_bind_int(stmt, 7i32, (*msg).state);
                    sqlite3_bind_text(
                        stmt,
                        8i32,
                        if !(*msg).text.is_null() {
                            (*msg).text
                        } else {
                            b"\x00" as *const u8 as *const libc::c_char
                        },
                        -1i32,
                        None,
                    );
                    sqlite3_bind_text(stmt, 9i32, (*(*msg).param).packed, -1i32, None);
                    sqlite3_bind_int(stmt, 10i32, (*msg).hidden);
                    sqlite3_bind_text(stmt, 11i32, new_in_reply_to, -1i32, None);
                    sqlite3_bind_text(stmt, 12i32, new_references, -1i32, None);
                    sqlite3_bind_int(stmt, 13i32, location_id as libc::c_int);
                    if sqlite3_step(stmt) != 101i32 {
                        dc_log_error(
                            context,
                            0i32,
                            b"Cannot send message, cannot insert to database.\x00" as *const u8
                                as *const libc::c_char,
                            (*chat).id,
                        );
                    } else {
                        msg_id = dc_sqlite3_get_rowid(
                            context,
                            &context.sql,
                            b"msgs\x00" as *const u8 as *const libc::c_char,
                            b"rfc724_mid\x00" as *const u8 as *const libc::c_char,
                            new_rfc724_mid,
                        )
                    }
                }
            }
        }
    }
    free(parent_rfc724_mid as *mut libc::c_void);
    free(parent_in_reply_to as *mut libc::c_void);
    free(parent_references as *mut libc::c_void);
    free(new_rfc724_mid as *mut libc::c_void);
    free(new_in_reply_to as *mut libc::c_void);
    free(new_references as *mut libc::c_void);
    sqlite3_finalize(stmt);

    msg_id
}

// TODO should return bool /rtn
unsafe fn get_parent_mime_headers(
    chat: *const Chat,
    parent_rfc724_mid: *mut *mut libc::c_char,
    parent_in_reply_to: *mut *mut libc::c_char,
    parent_references: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(chat.is_null()
        || parent_rfc724_mid.is_null()
        || parent_in_reply_to.is_null()
        || parent_references.is_null())
    {
        stmt =
            dc_sqlite3_prepare(
                (*chat).context, &(*chat).context.sql,
                               b"SELECT rfc724_mid, mime_in_reply_to, mime_references FROM msgs WHERE timestamp=(SELECT max(timestamp) FROM msgs WHERE chat_id=? AND from_id!=?);\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
        sqlite3_bind_int(stmt, 2i32, 1i32);
        if sqlite3_step(stmt) == 100i32 {
            *parent_rfc724_mid = dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char);
            *parent_in_reply_to = dc_strdup(sqlite3_column_text(stmt, 1i32) as *const libc::c_char);
            *parent_references = dc_strdup(sqlite3_column_text(stmt, 2i32) as *const libc::c_char);
            success = 1i32
        }
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        if 0 == success {
            stmt =
                dc_sqlite3_prepare(
                    (*chat).context, &(*chat).context.sql,
                                   b"SELECT rfc724_mid, mime_in_reply_to, mime_references FROM msgs WHERE timestamp=(SELECT min(timestamp) FROM msgs WHERE chat_id=? AND from_id==?);\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
            sqlite3_bind_int(stmt, 2i32, 1i32);
            if sqlite3_step(stmt) == 100i32 {
                *parent_rfc724_mid =
                    dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char);
                *parent_in_reply_to =
                    dc_strdup(sqlite3_column_text(stmt, 1i32) as *const libc::c_char);
                *parent_references =
                    dc_strdup(sqlite3_column_text(stmt, 2i32) as *const libc::c_char);
                success = 1i32
            }
        }
    }
    sqlite3_finalize(stmt);
    success
}

pub unsafe fn dc_chat_is_self_talk(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    dc_param_exists((*chat).param, 'K' as i32)
}

/*******************************************************************************
 * Sending messages
 ******************************************************************************/
// TODO should return bool /rtn
unsafe fn last_msg_in_chat_encrypted(
    context: &Context,
    sql: &SQLite,
    chat_id: uint32_t,
) -> libc::c_int {
    let mut last_is_encrypted: libc::c_int = 0i32;
    let  stmt: *mut sqlite3_stmt =
        dc_sqlite3_prepare(
            context,
            sql,
            b"SELECT param  FROM msgs  WHERE timestamp=(SELECT MAX(timestamp) FROM msgs WHERE chat_id=?)  ORDER BY id DESC;\x00"
                as *const u8 as *const libc::c_char);
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if sqlite3_step(stmt) == 100i32 {
        let msg_param: *mut dc_param_t = dc_param_new();
        dc_param_set_packed(
            msg_param,
            sqlite3_column_text(stmt, 0i32) as *mut libc::c_char,
        );
        if 0 != dc_param_exists(msg_param, 'c' as i32) {
            last_is_encrypted = 1i32
        }
        dc_param_unref(msg_param);
    }
    sqlite3_finalize(stmt);
    last_is_encrypted
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_update_param(chat: *mut Chat) -> libc::c_int {
    let success: libc::c_int;
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*chat).context,
        &(*chat).context.sql,
        b"UPDATE chats SET param=? WHERE id=?\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, (*(*chat).param).packed, -1i32, None);
    sqlite3_bind_int(stmt, 2i32, (*chat).id as libc::c_int);
    success = if sqlite3_step(stmt) == 101i32 {
        1i32
    } else {
        0i32
    };
    sqlite3_finalize(stmt);
    success
}

pub unsafe fn dc_is_contact_in_chat(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
) -> libc::c_int {
    /* this function works for group and for normal chats, however, it is more useful for group chats.
    DC_CONTACT_ID_SELF may be used to check, if the user itself is in a group chat (DC_CONTACT_ID_SELF is not added to normal chats) */
    let ret: libc::c_int;
    let stmt: *mut sqlite3_stmt;

    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT contact_id FROM chats_contacts WHERE chat_id=? AND contact_id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
    ret = if sqlite3_step(stmt) == 100i32 {
        1i32
    } else {
        0i32
    };

    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_unarchive_chat(context: &Context, chat_id: uint32_t) {
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"UPDATE chats SET archived=0 WHERE id=?\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

pub unsafe fn dc_send_msg<'a>(
    context: &'a Context,
    chat_id: uint32_t,
    msg: *mut dc_msg_t<'a>,
) -> uint32_t {
    if msg.is_null() {
        return 0i32 as uint32_t;
    }
    if (*msg).state != 18i32 {
        if 0 == prepare_msg_common(context, chat_id, msg) {
            return 0i32 as uint32_t;
        }
    } else {
        if chat_id != 0i32 as libc::c_uint && chat_id != (*msg).chat_id {
            return 0i32 as uint32_t;
        }
        dc_update_msg_state(context, (*msg).id, 20i32);
    }
    if 0 == dc_job_send_msg(context, (*msg).id) {
        return 0i32 as uint32_t;
    }
    context.call_cb(
        Event::MSGS_CHANGED,
        (*msg).chat_id as uintptr_t,
        (*msg).id as uintptr_t,
    );

    if 0 != dc_param_exists((*msg).param, DC_PARAM_SET_LATITUDE as libc::c_int) {
        context.call_cb(Event::LOCATION_CHANGED, DC_CONTACT_ID_SELF, 0);
    }

    if 0 == chat_id {
        let forwards: *mut libc::c_char =
            dc_param_get((*msg).param, 'P' as i32, 0 as *const libc::c_char);
        if !forwards.is_null() {
            let mut p: *mut libc::c_char = forwards;
            while 0 != *p {
                let id: int32_t = strtol(p, &mut p, 10i32) as int32_t;
                if 0 == id {
                    // avoid hanging if user tampers with db
                    break;
                } else {
                    let copy: *mut dc_msg_t = dc_get_msg(context, id as uint32_t);
                    if !copy.is_null() {
                        dc_send_msg(context, 0i32 as uint32_t, copy);
                    }
                    dc_msg_unref(copy);
                }
            }
            dc_param_set((*msg).param, 'P' as i32, 0 as *const libc::c_char);
            dc_msg_save_param_to_disk(msg);
        }
        free(forwards as *mut libc::c_void);
    }

    (*msg).id
}

pub unsafe fn dc_send_text_msg(
    context: &Context,
    chat_id: uint32_t,
    text_to_send: *const libc::c_char,
) -> uint32_t {
    let mut msg: *mut dc_msg_t = dc_msg_new(context, 10i32);
    let mut ret: uint32_t = 0i32 as uint32_t;
    if !(chat_id <= 9i32 as libc::c_uint || text_to_send.is_null()) {
        (*msg).text = dc_strdup(text_to_send);
        ret = dc_send_msg(context, chat_id, msg)
    }
    dc_msg_unref(msg);
    ret
}

pub unsafe fn dc_set_draft(context: &Context, chat_id: uint32_t, msg: *mut dc_msg_t) {
    if chat_id <= 9i32 as libc::c_uint {
        return;
    }
    if 0 != set_draft_raw(context, chat_id, msg) {
        context.call_cb(Event::MSGS_CHANGED, chat_id as uintptr_t, 0i32 as uintptr_t);
    };
}

// TODO should return bool /rtn
unsafe fn set_draft_raw(context: &Context, chat_id: uint32_t, msg: *mut dc_msg_t) -> libc::c_int {
    let current_block: u64;
    // similar to as dc_set_draft() but does not emit an event
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let prev_draft_msg_id: uint32_t;
    let mut sth_changed: libc::c_int = 0i32;
    prev_draft_msg_id = get_draft_msg_id(context, chat_id);
    if 0 != prev_draft_msg_id {
        dc_delete_msg_from_db(context, prev_draft_msg_id);
        sth_changed = 1i32
    }
    // save new draft
    if !msg.is_null() {
        if (*msg).type_0 == 10i32 {
            if (*msg).text.is_null() || *(*msg).text.offset(0isize) as libc::c_int == 0i32 {
                current_block = 14513523936503887211;
            } else {
                current_block = 4495394744059808450;
            }
        } else if (*msg).type_0 == 20i32
            || (*msg).type_0 == 21i32
            || (*msg).type_0 == 40i32
            || (*msg).type_0 == 41i32
            || (*msg).type_0 == 50i32
            || (*msg).type_0 == 60i32
        {
            pathNfilename = dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
            if pathNfilename.is_null() {
                current_block = 14513523936503887211;
            } else if 0 != dc_msg_is_increation(msg)
                && 0 == dc_is_blobdir_path(context, pathNfilename)
            {
                current_block = 14513523936503887211;
            } else if 0 == dc_make_rel_and_copy(context, &mut pathNfilename) {
                current_block = 14513523936503887211;
            } else {
                dc_param_set((*msg).param, 'f' as i32, pathNfilename);
                current_block = 4495394744059808450;
            }
        } else {
            current_block = 14513523936503887211;
        }
        match current_block {
            14513523936503887211 => {}
            _ => {
                stmt =
                    dc_sqlite3_prepare(
                        context,&context.sql,
                                       b"INSERT INTO msgs (chat_id, from_id, timestamp, type, state, txt, param, hidden) VALUES (?,?,?, ?,?,?,?,?);\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
                sqlite3_bind_int(stmt, 2i32, 1i32);
                sqlite3_bind_int64(stmt, 3i32, time() as sqlite3_int64);
                sqlite3_bind_int(stmt, 4i32, (*msg).type_0);
                sqlite3_bind_int(stmt, 5i32, 19i32);
                sqlite3_bind_text(
                    stmt,
                    6i32,
                    if !(*msg).text.is_null() {
                        (*msg).text
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                    -1i32,
                    None,
                );
                sqlite3_bind_text(stmt, 7i32, (*(*msg).param).packed, -1i32, None);
                sqlite3_bind_int(stmt, 8i32, 1i32);
                if !(sqlite3_step(stmt) != 101i32) {
                    sth_changed = 1i32
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    free(pathNfilename as *mut libc::c_void);
    sth_changed
}

unsafe fn get_draft_msg_id(context: &Context, chat_id: uint32_t) -> uint32_t {
    let mut draft_msg_id: uint32_t = 0i32 as uint32_t;
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT id FROM msgs WHERE chat_id=? AND state=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, 19i32);
    if sqlite3_step(stmt) == 100i32 {
        draft_msg_id = sqlite3_column_int(stmt, 0i32) as uint32_t
    }
    sqlite3_finalize(stmt);
    draft_msg_id
}

pub unsafe fn dc_get_draft(context: &Context, chat_id: uint32_t) -> *mut dc_msg_t {
    let draft_msg_id: uint32_t;
    let draft_msg: *mut dc_msg_t;
    if chat_id <= 9i32 as libc::c_uint {
        return 0 as *mut dc_msg_t;
    }
    draft_msg_id = get_draft_msg_id(context, chat_id);
    if draft_msg_id == 0i32 as libc::c_uint {
        return 0 as *mut dc_msg_t;
    }
    draft_msg = dc_msg_new_untyped(context);
    if !dc_msg_load_from_db(draft_msg, context, draft_msg_id) {
        dc_msg_unref(draft_msg);
        return 0 as *mut dc_msg_t;
    }

    draft_msg
}

pub unsafe fn dc_get_chat_msgs(
    context: &Context,
    chat_id: uint32_t,
    flags: uint32_t,
    marker1before: uint32_t,
) -> *mut dc_array_t {
    //clock_t       start = clock();
    let mut success: libc::c_int = 0i32;
    let ret: *mut dc_array_t = dc_array_new(512i32 as size_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut curr_id: uint32_t;
    let mut curr_local_timestamp: i64;
    let mut curr_day: libc::c_int;
    let mut last_day = 0;
    let cnv_to_local = dc_gm2local_offset();
    if !ret.is_null() {
        if chat_id == 1i32 as libc::c_uint {
            let show_emails: libc::c_int = dc_sqlite3_get_config_int(
                context,
                &context.sql,
                b"show_emails\x00" as *const u8 as *const libc::c_char,
                0i32,
            );
            stmt =
                dc_sqlite3_prepare(
                    context,&context.sql,
                                   b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN chats ON m.chat_id=chats.id LEFT JOIN contacts ON m.from_id=contacts.id WHERE m.from_id!=1   AND m.from_id!=2   AND m.hidden=0    AND chats.blocked=2   AND contacts.blocked=0   AND m.msgrmsg>=?  ORDER BY m.timestamp,m.id;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, if show_emails == 2i32 { 0i32 } else { 1i32 });
        } else if chat_id == 5i32 as libc::c_uint {
            stmt =
                dc_sqlite3_prepare(
                    context,&context.sql,
                                   b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id WHERE m.starred=1    AND m.hidden=0    AND ct.blocked=0 ORDER BY m.timestamp,m.id;\x00"
                                       as *const u8 as *const libc::c_char)
        } else {
            stmt =
                dc_sqlite3_prepare(
                    context,&context.sql,
                                   b"SELECT m.id, m.timestamp FROM msgs m WHERE m.chat_id=?    AND m.hidden=0  ORDER BY m.timestamp,m.id;\x00"
                                       as *const u8 as *const libc::c_char);
            sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        }
        while sqlite3_step(stmt) == 100i32 {
            curr_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
            if curr_id == marker1before {
                dc_array_add_id(ret, 1i32 as uint32_t);
            }
            if 0 != flags & 0x1i32 as libc::c_uint {
                curr_local_timestamp =
                    (sqlite3_column_int64(stmt, 1i32) as i64 + cnv_to_local) as i64;
                curr_day = (curr_local_timestamp / 86400) as libc::c_int;
                if curr_day != last_day {
                    dc_array_add_id(ret, 9i32 as uint32_t);
                    last_day = curr_day
                }
            }
            dc_array_add_id(ret, curr_id);
        }
        success = 1i32
    }
    sqlite3_finalize(stmt);
    if 0 != success {
        return ret;
    } else {
        if !ret.is_null() {
            dc_array_unref(ret);
        }
        return 0 as *mut dc_array_t;
    };
}

pub unsafe fn dc_get_msg_cnt(context: &Context, chat_id: uint32_t) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let stmt: *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT COUNT(*) FROM msgs WHERE chat_id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if !(sqlite3_step(stmt) != 100i32) {
        ret = sqlite3_column_int(stmt, 0i32);
    }

    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_get_fresh_msg_cnt(context: &Context, chat_id: uint32_t) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let stmt: *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT COUNT(*) FROM msgs  WHERE state=10   AND hidden=0    AND chat_id=?;\x00"
            as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if !(sqlite3_step(stmt) != 100i32) {
        ret = sqlite3_column_int(stmt, 0i32);
    }
    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_marknoticed_chat(context: &Context, chat_id: uint32_t) {
    let check: *mut sqlite3_stmt;
    let mut update: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;

    check = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT id FROM msgs  WHERE chat_id=? AND state=10;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(check, 1i32, chat_id as libc::c_int);
    if !(sqlite3_step(check) != 100i32) {
        update = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"UPDATE msgs    SET state=13 WHERE chat_id=? AND state=10;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int(update, 1i32, chat_id as libc::c_int);
        sqlite3_step(update);
        context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
    }

    sqlite3_finalize(check);
    sqlite3_finalize(update);
}

pub unsafe fn dc_marknoticed_all_chats(context: &Context) {
    let check: *mut sqlite3_stmt;
    let mut update: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;

    check = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT id FROM msgs  WHERE state=10;\x00" as *const u8 as *const libc::c_char,
    );
    if !(sqlite3_step(check) != 100i32) {
        update = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"UPDATE msgs    SET state=13 WHERE state=10;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_step(update);
        context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
    }

    sqlite3_finalize(check);
    sqlite3_finalize(update);
}

pub unsafe fn dc_get_chat_media(
    context: &Context,
    chat_id: uint32_t,
    msg_type: libc::c_int,
    msg_type2: libc::c_int,
    msg_type3: libc::c_int,
) -> *mut dc_array_t {
    let ret: *mut dc_array_t = dc_array_new(100i32 as size_t);
    let  stmt: *mut sqlite3_stmt =
        dc_sqlite3_prepare(
            context,&context.sql,
                           b"SELECT id FROM msgs WHERE chat_id=? AND (type=? OR type=? OR type=?) ORDER BY timestamp, id;\x00"
                               as *const u8 as *const libc::c_char);
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, msg_type);
    sqlite3_bind_int(
        stmt,
        3i32,
        if msg_type2 > 0i32 {
            msg_type2
        } else {
            msg_type
        },
    );
    sqlite3_bind_int(
        stmt,
        4i32,
        if msg_type3 > 0i32 {
            msg_type3
        } else {
            msg_type
        },
    );
    while sqlite3_step(stmt) == 100i32 {
        dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
    }
    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_get_next_media(
    context: &Context,
    curr_msg_id: uint32_t,
    dir: libc::c_int,
    msg_type: libc::c_int,
    msg_type2: libc::c_int,
    msg_type3: libc::c_int,
) -> uint32_t {
    let mut ret_msg_id: uint32_t = 0i32 as uint32_t;
    let msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut list: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut i: libc::c_int;
    let cnt: libc::c_int;

    if dc_msg_load_from_db(msg, context, curr_msg_id) {
        list = dc_get_chat_media(
            context,
            (*msg).chat_id,
            if msg_type > 0i32 {
                msg_type
            } else {
                (*msg).type_0
            },
            msg_type2,
            msg_type3,
        );
        if !list.is_null() {
            cnt = dc_array_get_cnt(list) as libc::c_int;
            i = 0i32;
            while i < cnt {
                if curr_msg_id == dc_array_get_id(list, i as size_t) {
                    if dir > 0i32 {
                        if i + 1i32 < cnt {
                            ret_msg_id = dc_array_get_id(list, (i + 1i32) as size_t)
                        }
                    } else if dir < 0i32 {
                        if i - 1i32 >= 0i32 {
                            ret_msg_id = dc_array_get_id(list, (i - 1i32) as size_t)
                        }
                    }
                    break;
                } else {
                    i += 1
                }
            }
        }
    }

    dc_array_unref(list);
    dc_msg_unref(msg);
    ret_msg_id
}

pub unsafe fn dc_archive_chat(context: &Context, chat_id: uint32_t, archive: libc::c_int) {
    if chat_id <= 9i32 as libc::c_uint || archive != 0i32 && archive != 1i32 {
        return;
    }
    if 0 != archive {
        let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"UPDATE msgs SET state=13 WHERE chat_id=? AND state=10;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
    }
    let stmt_0: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"UPDATE chats SET archived=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt_0, 1i32, archive);
    sqlite3_bind_int(stmt_0, 2i32, chat_id as libc::c_int);
    sqlite3_step(stmt_0);
    sqlite3_finalize(stmt_0);
    context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
}

pub unsafe fn dc_delete_chat(context: &Context, chat_id: uint32_t) {
    /* Up to 2017-11-02 deleting a group also implied leaving it, see above why we have changed this. */
    let obj: *mut Chat = dc_chat_new(context);
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(chat_id <= 9i32 as libc::c_uint) {
        if dc_chat_load_from_db(obj, chat_id) {
            q3 = sqlite3_mprintf(
                b"DELETE FROM msgs_mdns WHERE msg_id IN (SELECT id FROM msgs WHERE chat_id=%i);\x00"
                    as *const u8 as *const libc::c_char,
                chat_id,
            );
            if !(0 == dc_sqlite3_execute(context, &context.sql, q3)) {
                sqlite3_free(q3 as *mut libc::c_void);
                q3 = sqlite3_mprintf(
                    b"DELETE FROM msgs WHERE chat_id=%i;\x00" as *const u8 as *const libc::c_char,
                    chat_id,
                );
                if !(0 == dc_sqlite3_execute(context, &context.sql, q3)) {
                    sqlite3_free(q3 as *mut libc::c_void);
                    q3 = sqlite3_mprintf(
                        b"DELETE FROM chats_contacts WHERE chat_id=%i;\x00" as *const u8
                            as *const libc::c_char,
                        chat_id,
                    );
                    if !(0 == dc_sqlite3_execute(context, &context.sql, q3)) {
                        sqlite3_free(q3 as *mut libc::c_void);
                        q3 = sqlite3_mprintf(
                            b"DELETE FROM chats WHERE id=%i;\x00" as *const u8
                                as *const libc::c_char,
                            chat_id,
                        );
                        if !(0 == dc_sqlite3_execute(context, &context.sql, q3)) {
                            sqlite3_free(q3 as *mut libc::c_void);
                            q3 = 0 as *mut libc::c_char;
                            context.call_cb(
                                Event::MSGS_CHANGED,
                                0i32 as uintptr_t,
                                0i32 as uintptr_t,
                            );
                            dc_job_kill_action(context, 105i32);
                            dc_job_add(context, 105i32, 0i32, 0 as *const libc::c_char, 10i32);
                        }
                    }
                }
            }
        }
    }
    dc_chat_unref(obj);
    sqlite3_free(q3 as *mut libc::c_void);
}

pub unsafe fn dc_get_chat_contacts(context: &Context, chat_id: uint32_t) -> *mut dc_array_t {
    /* Normal chats do not include SELF.  Group chats do (as it may happen that one is deleted from a
    groupchat but the chats stays visible, moreover, this makes displaying lists easier) */
    let ret: *mut dc_array_t = dc_array_new(100i32 as size_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;

    if !(chat_id == 1i32 as libc::c_uint) {
        /* we could also create a list for all contacts in the deaddrop by searching contacts belonging to chats with chats.blocked=2, however, currently this is not needed */
        stmt =
            dc_sqlite3_prepare(
                context,&context.sql,
                               b"SELECT cc.contact_id FROM chats_contacts cc LEFT JOIN contacts c ON c.id=cc.contact_id WHERE cc.chat_id=? ORDER BY c.id=1, LOWER(c.name||c.addr), c.id;\x00"
                               as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        while sqlite3_step(stmt) == 100i32 {
            dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
        }
    }

    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_get_chat(context: &Context, chat_id: uint32_t) -> *mut Chat {
    let mut success: libc::c_int = 0i32;
    let obj: *mut Chat = dc_chat_new(context);

    if dc_chat_load_from_db(obj, chat_id) {
        success = 1i32
    }

    if 0 != success {
        return obj;
    } else {
        dc_chat_unref(obj);
        return 0 as *mut Chat;
    };
}

// handle group chats
pub unsafe fn dc_create_group_chat(
    context: &Context,
    verified: libc::c_int,
    chat_name: *const libc::c_char,
) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut draft_msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let grpid: *mut libc::c_char;
    let stmt: *mut sqlite3_stmt;
    if chat_name.is_null() || *chat_name.offset(0isize) as libc::c_int == 0i32 {
        return 0i32 as uint32_t;
    }
    let draft_txt =
        to_cstring(context.stock_str_repl_string(StockId::NewGroupDraft, as_str(chat_name)));
    grpid = dc_create_id();
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"INSERT INTO chats (type, name, grpid, param) VALUES(?, ?, ?, \'U=1\');\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, if 0 != verified { 130i32 } else { 120i32 });
    sqlite3_bind_text(stmt, 2i32, chat_name, -1i32, None);
    sqlite3_bind_text(stmt, 3i32, grpid, -1i32, None);
    if !(sqlite3_step(stmt) != 101i32) {
        chat_id = dc_sqlite3_get_rowid(
            context,
            &context.sql,
            b"chats\x00" as *const u8 as *const libc::c_char,
            b"grpid\x00" as *const u8 as *const libc::c_char,
            grpid,
        );
        if !(chat_id == 0i32 as libc::c_uint) {
            if !(0 == dc_add_to_chat_contacts_table(context, chat_id, 1i32 as uint32_t)) {
                draft_msg = dc_msg_new(context, 10i32);
                dc_msg_set_text(draft_msg, draft_txt.as_ptr());
                set_draft_raw(context, chat_id, draft_msg);
            }
        }
    }
    sqlite3_finalize(stmt);
    dc_msg_unref(draft_msg);
    free(grpid as *mut libc::c_void);
    if 0 != chat_id {
        context.call_cb(Event::MSGS_CHANGED, 0i32 as uintptr_t, 0i32 as uintptr_t);
    }

    chat_id
}

/* you MUST NOT modify this or the following strings */
// Context functions to work with chats
// TODO should return bool /rtn
pub unsafe fn dc_add_to_chat_contacts_table(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
) -> libc::c_int {
    /* add a contact to a chat; the function does not check the type or if any of the record exist or are already added to the chat! */
    let ret: libc::c_int;
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"INSERT INTO chats_contacts (chat_id, contact_id) VALUES(?, ?)\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    sqlite3_bind_int(stmt, 2i32, contact_id as libc::c_int);
    ret = if sqlite3_step(stmt) == 101i32 {
        1i32
    } else {
        0i32
    };
    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_add_contact_to_chat(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
) -> libc::c_int {
    dc_add_contact_to_chat_ex(context, chat_id, contact_id, 0i32)
}

// TODO should return bool /rtn
pub unsafe fn dc_add_contact_to_chat_ex(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
    flags: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let chat: *mut Chat = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(contact.is_null() || chat_id <= 9i32 as libc::c_uint) {
        dc_reset_gossiped_timestamp(context, chat_id);
        /*this also makes sure, not contacts are added to special or normal chats*/
        if !(0i32 == real_group_exists(context, chat_id)
            || !dc_real_contact_exists(context, contact_id) && contact_id != 1i32 as libc::c_uint
            || !dc_chat_load_from_db(chat, chat_id))
        {
            if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                dc_log_event(
                    context,
                    Event::ERROR_SELF_NOT_IN_GROUP,
                    0i32,
                    b"Cannot add contact to group; self not in group.\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                /* we shoud respect this - whatever we send to the group, it gets discarded anyway! */
                if 0 != flags & 0x1i32 && dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 1i32
                {
                    dc_param_set((*chat).param, 'U' as i32, 0 as *const libc::c_char);
                    dc_chat_update_param(chat);
                }
                self_addr = dc_sqlite3_get_config(
                    context,
                    &context.sql,
                    b"configured_addr\x00" as *const u8 as *const libc::c_char,
                    b"\x00" as *const u8 as *const libc::c_char,
                );
                if !(strcasecmp((*contact).addr, self_addr) == 0i32) {
                    /* ourself is added using DC_CONTACT_ID_SELF, do not add it explicitly. if SELF is not in the group, members cannot be added at all. */
                    if 0 != dc_is_contact_in_chat(context, chat_id, contact_id) {
                        if 0 == flags & 0x1i32 {
                            success = 1i32;
                            current_block = 12326129973959287090;
                        } else {
                            current_block = 15125582407903384992;
                        }
                    } else {
                        // else continue and send status mail
                        if (*chat).type_0 == 130i32 {
                            if dc_contact_is_verified(contact) != 2i32 {
                                dc_log_error(context, 0i32,
                                             b"Only bidirectional verified contacts can be added to verified groups.\x00"
                                                 as *const u8 as
                                                 *const libc::c_char);
                                current_block = 12326129973959287090;
                            } else {
                                current_block = 13472856163611868459;
                            }
                        } else {
                            current_block = 13472856163611868459;
                        }
                        match current_block {
                            12326129973959287090 => {}
                            _ => {
                                if 0i32
                                    == dc_add_to_chat_contacts_table(context, chat_id, contact_id)
                                {
                                    current_block = 12326129973959287090;
                                } else {
                                    current_block = 15125582407903384992;
                                }
                            }
                        }
                    }
                    match current_block {
                        12326129973959287090 => {}
                        _ => {
                            if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 0i32 {
                                (*msg).type_0 = 10i32;
                                let tmp = to_cstring(context.stock_system_msg(
                                    StockId::MsgAddMember,
                                    as_str((*contact).addr),
                                    "",
                                    DC_CONTACT_ID_SELF as u32,
                                ));
                                (*msg).text = dc_strdup(tmp.as_ptr());
                                dc_param_set_int((*msg).param, 'S' as i32, 4i32);
                                dc_param_set((*msg).param, 'E' as i32, (*contact).addr);
                                dc_param_set_int((*msg).param, 'F' as i32, flags);
                                (*msg).id = dc_send_msg(context, chat_id, msg);
                                context.call_cb(
                                    Event::MSGS_CHANGED,
                                    chat_id as uintptr_t,
                                    (*msg).id as uintptr_t,
                                );
                            }
                            context.call_cb(
                                Event::MSGS_CHANGED,
                                chat_id as uintptr_t,
                                0i32 as uintptr_t,
                            );
                            success = 1i32
                        }
                    }
                }
            }
        }
    }
    dc_chat_unref(chat);
    dc_contact_unref(contact);
    dc_msg_unref(msg);
    free(self_addr as *mut libc::c_void);

    success
}

// TODO should return bool /rtn
unsafe fn real_group_exists(context: &Context, chat_id: uint32_t) -> libc::c_int {
    // check if a group or a verified group exists under the given ID
    let stmt: *mut sqlite3_stmt;
    let mut ret: libc::c_int = 0i32;
    if !context.sql.is_open() || chat_id <= 9i32 as libc::c_uint {
        return 0i32;
    }
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT id FROM chats  WHERE id=?    AND (type=120 OR type=130);\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if sqlite3_step(stmt) == 100i32 {
        ret = 1i32
    }
    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_reset_gossiped_timestamp(context: &Context, chat_id: uint32_t) {
    dc_set_gossiped_timestamp(context, chat_id, 0);
}

pub unsafe fn dc_set_gossiped_timestamp(context: &Context, chat_id: uint32_t, timestamp: i64) {
    let stmt: *mut sqlite3_stmt;
    if 0 != chat_id {
        dc_log_info(
            context,
            0i32,
            b"set gossiped_timestamp for chat #%i to %i.\x00" as *const u8 as *const libc::c_char,
            chat_id as libc::c_int,
            timestamp as libc::c_int,
        );
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"UPDATE chats SET gossiped_timestamp=? WHERE id=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
        sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    } else {
        dc_log_info(
            context,
            0i32,
            b"set gossiped_timestamp for all chats to %i.\x00" as *const u8 as *const libc::c_char,
            timestamp as libc::c_int,
        );
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"UPDATE chats SET gossiped_timestamp=?;\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
    }
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

// TODO should return bool /rtn
pub unsafe fn dc_remove_contact_from_chat(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let chat: *mut Chat = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(chat_id <= 9i32 as libc::c_uint
        || contact_id <= 9i32 as libc::c_uint && contact_id != 1i32 as libc::c_uint)
    {
        /* we do not check if "contact_id" exists but just delete all records with the id from chats_contacts */
        /* this allows to delete pending references to deleted contacts.  Of course, this should _not_ happen. */
        if !(0i32 == real_group_exists(context, chat_id) || !dc_chat_load_from_db(chat, chat_id)) {
            if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                dc_log_event(
                    context,
                    Event::ERROR_SELF_NOT_IN_GROUP,
                    0i32,
                    b"Cannot remove contact from chat; self not in group.\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                /* we shoud respect this - whatever we send to the group, it gets discarded anyway! */
                if !contact.is_null() {
                    if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 0i32 {
                        (*msg).type_0 = 10i32;
                        if (*contact).id == 1i32 as libc::c_uint {
                            dc_set_group_explicitly_left(context, (*chat).grpid);
                            let tmp = to_cstring(context.stock_system_msg(
                                StockId::MsgGroupLeft,
                                "",
                                "",
                                DC_CONTACT_ID_SELF as u32,
                            ));
                            (*msg).text = dc_strdup(tmp.as_ptr())
                        } else {
                            let tmp = to_cstring(context.stock_system_msg(
                                StockId::MsgDelMember,
                                as_str((*contact).addr),
                                "",
                                DC_CONTACT_ID_SELF as u32,
                            ));
                            (*msg).text = dc_strdup(tmp.as_ptr())
                        }
                        dc_param_set_int((*msg).param, 'S' as i32, 5i32);
                        dc_param_set((*msg).param, 'E' as i32, (*contact).addr);
                        (*msg).id = dc_send_msg(context, chat_id, msg);
                        context.call_cb(
                            Event::MSGS_CHANGED,
                            chat_id as uintptr_t,
                            (*msg).id as uintptr_t,
                        );
                    }
                }
                q3 = sqlite3_mprintf(
                    b"DELETE FROM chats_contacts WHERE chat_id=%i AND contact_id=%i;\x00"
                        as *const u8 as *const libc::c_char,
                    chat_id,
                    contact_id,
                );
                if !(0 == dc_sqlite3_execute(context, &context.sql, q3)) {
                    context.call_cb(
                        Event::CHAT_MODIFIED,
                        chat_id as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    success = 1i32
                }
            }
        }
    }
    sqlite3_free(q3 as *mut libc::c_void);
    dc_chat_unref(chat);
    dc_contact_unref(contact);
    dc_msg_unref(msg);

    success
}

pub unsafe fn dc_set_group_explicitly_left(context: &Context, grpid: *const libc::c_char) {
    if 0 == dc_is_group_explicitly_left(context, grpid) {
        let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"INSERT INTO leftgrps (grpid) VALUES(?);\x00" as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1i32, grpid, -1i32, None);
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
    };
}

// TODO should return bool /rtn
pub unsafe fn dc_is_group_explicitly_left(
    context: &Context,
    grpid: *const libc::c_char,
) -> libc::c_int {
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT id FROM leftgrps WHERE grpid=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, grpid, -1i32, None);
    let ret: libc::c_int = (sqlite3_step(stmt) == 100i32) as libc::c_int;
    sqlite3_finalize(stmt);
    ret
}

// TODO should return bool /rtn
pub unsafe fn dc_set_chat_name(
    context: &Context,
    chat_id: uint32_t,
    new_name: *const libc::c_char,
) -> libc::c_int {
    /* the function only sets the names of group chats; normal chats get their names from the contacts */
    let mut success: libc::c_int = 0i32;
    let chat: *mut Chat = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(new_name.is_null()
        || *new_name.offset(0isize) as libc::c_int == 0i32
        || chat_id <= 9i32 as libc::c_uint)
    {
        if !(0i32 == real_group_exists(context, chat_id) || !dc_chat_load_from_db(chat, chat_id)) {
            if strcmp((*chat).name, new_name) == 0i32 {
                success = 1i32
            } else if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                dc_log_event(
                    context,
                    Event::ERROR_SELF_NOT_IN_GROUP,
                    0i32,
                    b"Cannot set chat name; self not in group\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                /* we shoud respect this - whatever we send to the group, it gets discarded anyway! */
                q3 = sqlite3_mprintf(
                    b"UPDATE chats SET name=%Q WHERE id=%i;\x00" as *const u8
                        as *const libc::c_char,
                    new_name,
                    chat_id,
                );
                if !(0 == dc_sqlite3_execute(context, &context.sql, q3)) {
                    if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 0i32 {
                        (*msg).type_0 = 10i32;
                        let tmp = to_cstring(context.stock_system_msg(
                            StockId::MsgGrpName,
                            as_str((*chat).name),
                            as_str(new_name),
                            DC_CONTACT_ID_SELF as u32,
                        ));
                        (*msg).text = dc_strdup(tmp.as_ptr());
                        dc_param_set_int((*msg).param, 'S' as i32, 2i32);
                        dc_param_set((*msg).param, 'E' as i32, (*chat).name);
                        (*msg).id = dc_send_msg(context, chat_id, msg);
                        context.call_cb(
                            Event::MSGS_CHANGED,
                            chat_id as uintptr_t,
                            (*msg).id as uintptr_t,
                        );
                    }
                    context.call_cb(
                        Event::CHAT_MODIFIED,
                        chat_id as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    success = 1i32
                }
            }
        }
    }

    sqlite3_free(q3 as *mut libc::c_void);
    dc_chat_unref(chat);
    dc_msg_unref(msg);

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_set_chat_profile_image(
    context: &Context,
    chat_id: uint32_t,
    new_image: *const libc::c_char,
) -> libc::c_int {
    let current_block: u64;
    let mut success: libc::c_int = 0i32;
    let chat: *mut Chat = dc_chat_new(context);
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut new_image_rel: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(chat_id <= 9i32 as libc::c_uint) {
        if !(0i32 == real_group_exists(context, chat_id) || !dc_chat_load_from_db(chat, chat_id)) {
            if !(dc_is_contact_in_chat(context, chat_id, 1i32 as uint32_t) == 1i32) {
                dc_log_event(
                    context,
                    Event::ERROR_SELF_NOT_IN_GROUP,
                    0i32,
                    b"Cannot set chat profile image; self not in group.\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                /* we shoud respect this - whatever we send to the group, it gets discarded anyway! */
                if !new_image.is_null() {
                    new_image_rel = dc_strdup(new_image);
                    if 0 == dc_make_rel_and_copy(context, &mut new_image_rel) {
                        current_block = 14766584022300871387;
                    } else {
                        current_block = 1856101646708284338;
                    }
                } else {
                    current_block = 1856101646708284338;
                }
                match current_block {
                    14766584022300871387 => {}
                    _ => {
                        dc_param_set((*chat).param, 'i' as i32, new_image_rel);
                        if !(0 == dc_chat_update_param(chat)) {
                            if dc_param_get_int((*chat).param, 'U' as i32, 0i32) == 0i32 {
                                dc_param_set_int((*msg).param, 'S' as i32, 3i32);
                                dc_param_set((*msg).param, 'E' as i32, new_image_rel);
                                (*msg).type_0 = 10i32;
                                let tmp = to_cstring(context.stock_system_msg(
                                    if !new_image_rel.is_null() {
                                        StockId::MsgGrpImgChanged
                                    } else {
                                        StockId::MsgGrpImgDeleted
                                    },
                                    "",
                                    "",
                                    DC_CONTACT_ID_SELF as u32,
                                ));
                                (*msg).text = dc_strdup(tmp.as_ptr());
                                (*msg).id = dc_send_msg(context, chat_id, msg);
                                context.call_cb(
                                    Event::MSGS_CHANGED,
                                    chat_id as uintptr_t,
                                    (*msg).id as uintptr_t,
                                );
                            }
                            context.call_cb(
                                Event::CHAT_MODIFIED,
                                chat_id as uintptr_t,
                                0i32 as uintptr_t,
                            );
                            success = 1i32
                        }
                    }
                }
            }
        }
    }

    dc_chat_unref(chat);
    dc_msg_unref(msg);
    free(new_image_rel as *mut libc::c_void);

    success
}

pub unsafe fn dc_forward_msgs(
    context: &Context,
    msg_ids: *const uint32_t,
    msg_cnt: libc::c_int,
    chat_id: uint32_t,
) {
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let chat: *mut Chat = dc_chat_new(context);
    let contact: *mut dc_contact_t = dc_contact_new(context);
    let created_db_entries: *mut carray = carray_new(16i32 as libc::c_uint);
    let mut idsstr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut q3: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut curr_timestamp: i64;
    let original_param: *mut dc_param_t = dc_param_new();
    if !(msg_ids.is_null() || msg_cnt <= 0i32 || chat_id <= 9i32 as libc::c_uint) {
        dc_unarchive_chat(context, chat_id);
        if dc_chat_load_from_db(chat, chat_id) {
            curr_timestamp = dc_create_smeared_timestamps(context, msg_cnt);
            idsstr = dc_arr_to_string(msg_ids, msg_cnt);
            q3 = sqlite3_mprintf(
                b"SELECT id FROM msgs WHERE id IN(%s) ORDER BY timestamp,id\x00" as *const u8
                    as *const libc::c_char,
                idsstr,
            );
            stmt = dc_sqlite3_prepare(context, &context.sql, q3);
            loop {
                if !(sqlite3_step(stmt) == 100i32) {
                    break;
                }
                let src_msg_id: libc::c_int = sqlite3_column_int(stmt, 0i32);
                if !dc_msg_load_from_db(msg, context, src_msg_id as uint32_t) {
                    break;
                }
                dc_param_set_packed(original_param, (*(*msg).param).packed);
                if (*msg).from_id != 1i32 as libc::c_uint {
                    dc_param_set_int((*msg).param, 'a' as i32, 1i32);
                }
                dc_param_set((*msg).param, 'c' as i32, 0 as *const libc::c_char);
                dc_param_set((*msg).param, 'u' as i32, 0 as *const libc::c_char);
                dc_param_set((*msg).param, 'S' as i32, 0 as *const libc::c_char);
                let new_msg_id: uint32_t;
                if (*msg).state == 18i32 {
                    let fresh9 = curr_timestamp;
                    curr_timestamp = curr_timestamp + 1;
                    new_msg_id = prepare_msg_raw(context, chat, msg, fresh9);
                    let save_param: *mut dc_param_t = (*msg).param;
                    (*msg).param = original_param;
                    (*msg).id = src_msg_id as uint32_t;
                    let old_fwd: *mut libc::c_char = dc_param_get(
                        (*msg).param,
                        'P' as i32,
                        b"\x00" as *const u8 as *const libc::c_char,
                    );
                    let new_fwd: *mut libc::c_char = dc_mprintf(
                        b"%s %d\x00" as *const u8 as *const libc::c_char,
                        old_fwd,
                        new_msg_id,
                    );
                    dc_param_set((*msg).param, 'P' as i32, new_fwd);
                    dc_msg_save_param_to_disk(msg);
                    free(new_fwd as *mut libc::c_void);
                    free(old_fwd as *mut libc::c_void);
                    (*msg).param = save_param
                } else {
                    (*msg).state = 20i32;
                    let fresh10 = curr_timestamp;
                    curr_timestamp = curr_timestamp + 1;
                    new_msg_id = prepare_msg_raw(context, chat, msg, fresh10);
                    dc_job_send_msg(context, new_msg_id);
                }
                carray_add(
                    created_db_entries,
                    chat_id as uintptr_t as *mut libc::c_void,
                    0 as *mut libc::c_uint,
                );
                carray_add(
                    created_db_entries,
                    new_msg_id as uintptr_t as *mut libc::c_void,
                    0 as *mut libc::c_uint,
                );
            }
        }
    }
    if !created_db_entries.is_null() {
        let mut i = 0u32;
        let icnt = carray_count(created_db_entries);
        while i < icnt {
            context.call_cb(
                Event::MSGS_CHANGED,
                carray_get(created_db_entries, i) as uintptr_t,
                carray_get(created_db_entries, i.wrapping_add(1)) as uintptr_t,
            );
            i = i.wrapping_add(2);
        }
        carray_free(created_db_entries);
    }
    dc_contact_unref(contact);
    dc_msg_unref(msg);
    dc_chat_unref(chat);
    sqlite3_finalize(stmt);
    free(idsstr as *mut libc::c_void);
    sqlite3_free(q3 as *mut libc::c_void);
    dc_param_unref(original_param);
}

pub unsafe fn dc_chat_get_id(chat: *const Chat) -> uint32_t {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32 as uint32_t;
    }
    (*chat).id
}

pub unsafe fn dc_chat_get_type(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    (*chat).type_0
}

pub unsafe fn dc_chat_get_name(chat: *const Chat) -> *mut libc::c_char {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char);
    }
    dc_strdup((*chat).name)
}

pub unsafe fn dc_chat_get_subtitle(chat: *const Chat) -> *mut libc::c_char {
    /* returns either the address or the number of chat members */
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char);
    }
    if (*chat).type_0 == 100i32 && 0 != dc_param_exists((*chat).param, 'K' as i32) {
        let tmp = to_cstring((*chat).context.stock_str(StockId::SelfTalk_SubTitle));
        ret = dc_strdup(tmp.as_ptr())
    } else if (*chat).type_0 == 100i32 {
        let r: libc::c_int;
        let  stmt: *mut sqlite3_stmt =
            dc_sqlite3_prepare(
                (*chat).context, &(*chat).context.sql,
                               b"SELECT c.addr FROM chats_contacts cc  LEFT JOIN contacts c ON c.id=cc.contact_id  WHERE cc.chat_id=?;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, (*chat).id as libc::c_int);
        r = sqlite3_step(stmt);
        if r == 100i32 {
            ret = dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char)
        }
        sqlite3_finalize(stmt);
    } else if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
        let cnt: libc::c_int;
        if (*chat).id == 1i32 as libc::c_uint {
            let tmp = to_cstring((*chat).context.stock_str(StockId::DeadDrop));
            ret = dc_strdup(tmp.as_ptr());
        } else {
            cnt = dc_get_chat_contact_cnt((*chat).context, (*chat).id);
            let tmp = to_cstring((*chat).context.stock_str_repl_int(StockId::Member, cnt));
            ret = dc_strdup(tmp.as_ptr());
        }
    }
    return if !ret.is_null() {
        ret
    } else {
        dc_strdup(b"Err\x00" as *const u8 as *const libc::c_char)
    };
}

pub unsafe fn dc_get_chat_contact_cnt(context: &Context, chat_id: uint32_t) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT COUNT(*) FROM chats_contacts WHERE chat_id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if sqlite3_step(stmt) == 100i32 {
        ret = sqlite3_column_int(stmt, 0i32)
    }
    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_chat_get_profile_image(chat: *const Chat) -> *mut libc::c_char {
    let mut image_rel: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut image_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut contacts: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !(chat.is_null() || (*chat).magic != 0xc4a7c4a7u32) {
        image_rel = dc_param_get((*chat).param, 'i' as i32, 0 as *const libc::c_char);
        if !image_rel.is_null() && 0 != *image_rel.offset(0isize) as libc::c_int {
            image_abs = dc_get_abs_path((*chat).context, image_rel)
        } else if (*chat).type_0 == 100i32 {
            contacts = dc_get_chat_contacts((*chat).context, (*chat).id);
            if (*contacts).count >= 1 {
                contact = dc_get_contact(
                    (*chat).context,
                    *(*contacts).array.offset(0isize) as uint32_t,
                );
                image_abs = dc_contact_get_profile_image(contact)
            }
        }
    }

    free(image_rel as *mut libc::c_void);
    dc_array_unref(contacts);
    dc_contact_unref(contact);

    image_abs
}

pub unsafe fn dc_chat_get_color(chat: *const Chat) -> uint32_t {
    let mut color: uint32_t = 0i32 as uint32_t;
    let mut contacts: *mut dc_array_t = 0 as *mut dc_array_t;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !(chat.is_null() || (*chat).magic != 0xc4a7c4a7u32) {
        if (*chat).type_0 == 100i32 {
            contacts = dc_get_chat_contacts((*chat).context, (*chat).id);
            if (*contacts).count >= 1 {
                contact = dc_get_contact(
                    (*chat).context,
                    *(*contacts).array.offset(0isize) as uint32_t,
                );
                color = dc_str_to_color((*contact).addr) as uint32_t
            }
        } else {
            color = dc_str_to_color((*chat).name) as uint32_t
        }
    }

    dc_array_unref(contacts);
    dc_contact_unref(contact);

    color
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_get_archived(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    (*chat).archived
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_is_unpromoted(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    dc_param_get_int((*chat).param, 'U' as i32, 0i32)
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_is_verified(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    ((*chat).type_0 == 130i32) as libc::c_int
}

// TODO should return bool /rtn
pub unsafe fn dc_chat_is_sending_locations(chat: *const Chat) -> libc::c_int {
    if chat.is_null() || (*chat).magic != 0xc4a7c4a7u32 {
        return 0i32;
    }
    (*chat).is_sending_locations
}

pub unsafe fn dc_get_chat_cnt(context: &Context) -> size_t {
    let mut ret: size_t = 0i32 as size_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if context.sql.is_open() {
        /* no database, no chats - this is no error (needed eg. for information) */
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"SELECT COUNT(*) FROM chats WHERE id>9 AND blocked=0;\x00" as *const u8
                as *const libc::c_char,
        );
        if !(sqlite3_step(stmt) != 100i32) {
            ret = sqlite3_column_int(stmt, 0i32) as size_t
        }
    }
    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_get_chat_id_by_grpid(
    context: &Context,
    grpid: *const libc::c_char,
    ret_blocked: *mut libc::c_int,
    ret_verified: *mut libc::c_int,
) -> uint32_t {
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let stmt: *mut sqlite3_stmt;
    if !ret_blocked.is_null() {
        *ret_blocked = 0i32
    }
    if !ret_verified.is_null() {
        *ret_verified = 0i32
    }

    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT id, blocked, type FROM chats WHERE grpid=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_text(stmt, 1i32, grpid, -1i32, None);
    if sqlite3_step(stmt) == 100i32 {
        chat_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
        if !ret_blocked.is_null() {
            *ret_blocked = sqlite3_column_int(stmt, 1i32)
        }
        if !ret_verified.is_null() {
            *ret_verified = (sqlite3_column_int(stmt, 2i32) == 130i32) as libc::c_int
        }
    }

    sqlite3_finalize(stmt);
    chat_id
}

pub unsafe fn dc_add_device_msg(context: &Context, chat_id: uint32_t, text: *const libc::c_char) {
    let msg_id: uint32_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let rfc724_mid: *mut libc::c_char = dc_create_outgoing_rfc724_mid(
        0 as *const libc::c_char,
        b"@device\x00" as *const u8 as *const libc::c_char,
    );
    if !text.is_null() {
        stmt =
            dc_sqlite3_prepare(
                context,&context.sql,
                               b"INSERT INTO msgs (chat_id,from_id,to_id, timestamp,type,state, txt,rfc724_mid) VALUES (?,?,?, ?,?,?, ?,?);\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
        sqlite3_bind_int(stmt, 2i32, 2i32);
        sqlite3_bind_int(stmt, 3i32, 2i32);
        sqlite3_bind_int64(
            stmt,
            4i32,
            dc_create_smeared_timestamp(context) as sqlite3_int64,
        );
        sqlite3_bind_int(stmt, 5i32, 10i32);
        sqlite3_bind_int(stmt, 6i32, 13i32);
        sqlite3_bind_text(stmt, 7i32, text, -1i32, None);
        sqlite3_bind_text(stmt, 8i32, rfc724_mid, -1i32, None);
        if !(sqlite3_step(stmt) != 101i32) {
            msg_id = dc_sqlite3_get_rowid(
                context,
                &context.sql,
                b"msgs\x00" as *const u8 as *const libc::c_char,
                b"rfc724_mid\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            context.call_cb(
                Event::MSGS_CHANGED,
                chat_id as uintptr_t,
                msg_id as uintptr_t,
            );
        }
    }
    free(rfc724_mid as *mut libc::c_void);
    sqlite3_finalize(stmt);
}
