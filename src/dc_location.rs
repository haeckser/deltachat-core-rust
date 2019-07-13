use crate::constants::Event;
use crate::context::*;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_job::*;
use crate::dc_log::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_saxparser::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

// location handling
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_location_t {
    pub location_id: uint32_t,
    pub latitude: libc::c_double,
    pub longitude: libc::c_double,
    pub accuracy: libc::c_double,
    pub timestamp: i64,
    pub contact_id: uint32_t,
    pub msg_id: uint32_t,
    pub chat_id: uint32_t,
    pub marker: *mut libc::c_char,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_kml_t {
    pub addr: *mut libc::c_char,
    pub locations: *mut dc_array_t,
    pub tag: libc::c_int,
    pub curr: dc_location_t,
}

// location streaming
pub unsafe fn dc_send_locations_to_chat(
    context: &Context,
    chat_id: uint32_t,
    seconds: libc::c_int,
) {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let now = time();
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let is_sending_locations_before: libc::c_int;
    if !(seconds < 0i32 || chat_id <= 9i32 as libc::c_uint) {
        is_sending_locations_before = dc_is_sending_locations_to_chat(context, chat_id);
        stmt =
            dc_sqlite3_prepare(
                context,
                &context.sql,
                b"UPDATE chats    SET locations_send_begin=?,        locations_send_until=?  WHERE id=?\x00"
                    as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(
            stmt,
            1i32,
            (if 0 != seconds { now } else { 0 }) as sqlite3_int64,
        );
        sqlite3_bind_int64(
            stmt,
            2i32,
            (if 0 != seconds {
                now + seconds as i64
            } else {
                0
            }) as sqlite3_int64,
        );
        sqlite3_bind_int(stmt, 3i32, chat_id as libc::c_int);
        sqlite3_step(stmt);
        if 0 != seconds && 0 == is_sending_locations_before {
            msg = dc_msg_new(context, 10i32);
            let tmp = to_cstring(context.stock_system_msg(StockId::MsgLocationEnabled, "", "", 0));
            (*msg).text = dc_strdup(tmp.as_ptr());
            dc_param_set_int((*msg).param, 'S' as i32, 8i32);
            dc_send_msg(context, chat_id, msg);
        } else if 0 == seconds && 0 != is_sending_locations_before {
            let stock_str =
                to_cstring(context.stock_system_msg(StockId::MsgLocationDisabled, "", "", 0));
            dc_add_device_msg(context, chat_id, stock_str.as_ptr());
        }
        context.call_cb(
            Event::CHAT_MODIFIED,
            chat_id as uintptr_t,
            0i32 as uintptr_t,
        );
        if 0 != seconds {
            schedule_MAYBE_SEND_LOCATIONS(context, 0i32);
            dc_job_add(
                context,
                5007i32,
                chat_id as libc::c_int,
                0 as *const libc::c_char,
                seconds + 1i32,
            );
        }
    }
    dc_msg_unref(msg);
    sqlite3_finalize(stmt);
}

/*******************************************************************************
 * job to send locations out to all chats that want them
 ******************************************************************************/
unsafe fn schedule_MAYBE_SEND_LOCATIONS(context: &Context, flags: libc::c_int) {
    if 0 != flags & 0x1i32 || 0 == dc_job_action_exists(context, 5005i32) {
        dc_job_add(context, 5005i32, 0i32, 0 as *const libc::c_char, 60i32);
    };
}

pub unsafe fn dc_is_sending_locations_to_chat(context: &Context, chat_id: uint32_t) -> libc::c_int {
    let mut is_sending_locations: libc::c_int = 0i32;
    let stmt: *mut sqlite3_stmt;

    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT id  FROM chats  WHERE (? OR id=?)   AND locations_send_until>?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(
        stmt,
        1i32,
        if chat_id == 0i32 as libc::c_uint {
            1i32
        } else {
            0i32
        },
    );
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_bind_int64(stmt, 3i32, time() as sqlite3_int64);
    if !(sqlite3_step(stmt) != 100i32) {
        is_sending_locations = 1i32
    }

    sqlite3_finalize(stmt);

    is_sending_locations
}

pub unsafe fn dc_set_location(
    context: &Context,
    latitude: libc::c_double,
    longitude: libc::c_double,
    accuracy: libc::c_double,
) -> libc::c_int {
    let mut stmt_chats: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut stmt_insert: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut continue_streaming: libc::c_int = 0i32;
    if latitude == 0.0f64 && longitude == 0.0f64 {
        continue_streaming = 1i32
    } else {
        stmt_chats = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"SELECT id FROM chats WHERE locations_send_until>?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_int64(stmt_chats, 1i32, time() as sqlite3_int64);
        while sqlite3_step(stmt_chats) == 100i32 {
            let chat_id: uint32_t = sqlite3_column_int(stmt_chats, 0i32) as uint32_t;
            stmt_insert =
                dc_sqlite3_prepare(
                    context,
                    &context.sql,
                    b"INSERT INTO locations  (latitude, longitude, accuracy, timestamp, chat_id, from_id) VALUES (?,?,?,?,?,?);\x00"
                        as *const u8 as *const libc::c_char);
            sqlite3_bind_double(stmt_insert, 1i32, latitude);
            sqlite3_bind_double(stmt_insert, 2i32, longitude);
            sqlite3_bind_double(stmt_insert, 3i32, accuracy);
            sqlite3_bind_int64(stmt_insert, 4i32, time() as sqlite3_int64);
            sqlite3_bind_int(stmt_insert, 5i32, chat_id as libc::c_int);
            sqlite3_bind_int(stmt_insert, 6i32, 1i32);
            sqlite3_step(stmt_insert);
            continue_streaming = 1i32
        }
        if 0 != continue_streaming {
            context.call_cb(
                Event::LOCATION_CHANGED,
                1i32 as uintptr_t,
                0i32 as uintptr_t,
            );
            schedule_MAYBE_SEND_LOCATIONS(context, 0i32);
        }
    }
    sqlite3_finalize(stmt_chats);
    sqlite3_finalize(stmt_insert);

    continue_streaming
}

pub unsafe fn dc_get_locations(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
    timestamp_from: i64,
    mut timestamp_to: i64,
) -> *mut dc_array_t {
    let ret: *mut dc_array_t = dc_array_new_typed(1i32, 500i32 as size_t);
    let stmt: *mut sqlite3_stmt;

    if timestamp_to == 0 {
        timestamp_to = time() + 10;
    }
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT l.id, l.latitude, l.longitude, l.accuracy, l.timestamp, l.independent, \
              m.id, l.from_id, l.chat_id, m.txt \
              FROM locations l  LEFT JOIN msgs m ON l.id=m.location_id  WHERE (? OR l.chat_id=?) \
              AND (? OR l.from_id=?) \
              AND (l.independent=1 OR (l.timestamp>=? AND l.timestamp<=?)) \
              ORDER BY l.timestamp DESC, l.id DESC, m.id DESC;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(
        stmt,
        1i32,
        if chat_id == 0i32 as libc::c_uint {
            1i32
        } else {
            0i32
        },
    );
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_bind_int(
        stmt,
        3i32,
        if contact_id == 0i32 as libc::c_uint {
            1i32
        } else {
            0i32
        },
    );
    sqlite3_bind_int(stmt, 4i32, contact_id as libc::c_int);
    sqlite3_bind_int(stmt, 5i32, timestamp_from as libc::c_int);
    sqlite3_bind_int(stmt, 6i32, timestamp_to as libc::c_int);
    while sqlite3_step(stmt) == 100i32 {
        let mut loc: *mut _dc_location =
            calloc(1, ::std::mem::size_of::<_dc_location>()) as *mut _dc_location;
        if loc.is_null() {
            break;
        }
        (*loc).location_id = sqlite3_column_double(stmt, 0i32) as uint32_t;
        (*loc).latitude = sqlite3_column_double(stmt, 1i32);
        (*loc).longitude = sqlite3_column_double(stmt, 2i32);
        (*loc).accuracy = sqlite3_column_double(stmt, 3i32);
        (*loc).timestamp = sqlite3_column_int64(stmt, 4i32) as i64;
        (*loc).independent = sqlite3_column_int(stmt, 5i32) as uint32_t;
        (*loc).msg_id = sqlite3_column_int(stmt, 6i32) as uint32_t;
        (*loc).contact_id = sqlite3_column_int(stmt, 7i32) as uint32_t;
        (*loc).chat_id = sqlite3_column_int(stmt, 8i32) as uint32_t;

        if 0 != (*loc).msg_id {
            let txt: *const libc::c_char = sqlite3_column_text(stmt, 9i32) as *const libc::c_char;
            if 0 != is_marker(txt) {
                (*loc).marker = strdup(txt)
            }
        }
        dc_array_add_ptr(ret, loc as *mut libc::c_void);
    }

    sqlite3_finalize(stmt);

    ret
}

// TODO should be bool /rtn
unsafe fn is_marker(txt: *const libc::c_char) -> libc::c_int {
    if !txt.is_null() {
        let len: libc::c_int = dc_utf8_strlen(txt) as libc::c_int;
        if len == 1i32 && *txt.offset(0isize) as libc::c_int != ' ' as i32 {
            return 1i32;
        }
    }

    0
}

pub unsafe fn dc_delete_all_locations(context: &Context) {
    let stmt: *mut sqlite3_stmt;

    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"DELETE FROM locations;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_step(stmt);
    context.call_cb(
        Event::LOCATION_CHANGED,
        0i32 as uintptr_t,
        0i32 as uintptr_t,
    );

    sqlite3_finalize(stmt);
}

pub unsafe fn dc_get_location_kml(
    context: &Context,
    chat_id: uint32_t,
    last_added_location_id: *mut uint32_t,
) -> *mut libc::c_char {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt;
    let self_addr: *mut libc::c_char;
    let now = time();
    let locations_send_begin: i64;
    let locations_send_until: i64;
    let locations_last_sent: i64;
    let mut location_count: libc::c_int = 0i32;
    let mut ret = String::new();

    self_addr = dc_sqlite3_get_config(
        context,
        &context.sql,
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        b"\x00" as *const u8 as *const libc::c_char,
    );
    stmt =
        dc_sqlite3_prepare(
            context,
            &context.sql,
            b"SELECT locations_send_begin, locations_send_until, locations_last_sent  FROM chats  WHERE id=?;\x00"
                as *const u8 as *const libc::c_char);
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if !(sqlite3_step(stmt) != 100i32) {
        locations_send_begin = sqlite3_column_int64(stmt, 0i32) as i64;
        locations_send_until = sqlite3_column_int64(stmt, 1i32) as i64;
        locations_last_sent = sqlite3_column_int64(stmt, 2i32) as i64;
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;

        if !(locations_send_begin == 0 || now > locations_send_until) {
            ret += &format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document addr=\"{}\">\n",
                to_string(self_addr),
            );
            stmt = dc_sqlite3_prepare(
                context,
                    &context.sql,
                    b"SELECT id, latitude, longitude, accuracy, timestamp\
                          FROM locations  WHERE from_id=? \
                          AND timestamp>=? \
                          AND (timestamp>=? OR timestamp=(SELECT MAX(timestamp) FROM locations WHERE from_id=?)) \
                          AND independent=0 \
                          GROUP BY timestamp \
                          ORDER BY timestamp;\x00" as *const u8
                        as *const libc::c_char,
                );

            sqlite3_bind_int(stmt, 1i32, 1i32);
            sqlite3_bind_int64(stmt, 2i32, locations_send_begin as sqlite3_int64);
            sqlite3_bind_int64(stmt, 3i32, locations_last_sent as sqlite3_int64);
            sqlite3_bind_int(stmt, 4i32, 1i32);
            while sqlite3_step(stmt) == 100i32 {
                let location_id: uint32_t = sqlite3_column_int(stmt, 0i32) as uint32_t;
                let latitude = sqlite3_column_double(stmt, 1i32);
                let longitude = sqlite3_column_double(stmt, 2i32);
                let accuracy = sqlite3_column_double(stmt, 3i32);
                let timestamp = get_kml_timestamp(sqlite3_column_int64(stmt, 4i32) as i64);
                ret += &format!(
                    "<Placemark><Timestamp><when>{}</when></Timestamp><Point><coordinates accuracy=\"{}\">{},{}</coordinates></Point></Placemark>\n\x00",
                    as_str(timestamp),
                    accuracy,
                    longitude,
                    latitude
                );
                location_count += 1;
                if !last_added_location_id.is_null() {
                    *last_added_location_id = location_id
                }
                free(timestamp as *mut libc::c_void);
            }
            if !(location_count == 0) {
                ret += "</Document>\n</kml>";
                success = 1;
            }
        }
    }

    sqlite3_finalize(stmt);
    free(self_addr as *mut libc::c_void);

    if 0 != success {
        strdup(to_cstring(ret).as_ptr())
    } else {
        0 as *mut libc::c_char
    }
}

/*******************************************************************************
 * create kml-files
 ******************************************************************************/
unsafe fn get_kml_timestamp(utc: i64) -> *mut libc::c_char {
    // Returns a string formatted as YYYY-MM-DDTHH:MM:SSZ. The trailing `Z` indicates UTC.
    let res = chrono::NaiveDateTime::from_timestamp(utc, 0)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    strdup(to_cstring(res).as_ptr())
}

pub unsafe fn dc_get_message_kml(
    timestamp: i64,
    latitude: libc::c_double,
    longitude: libc::c_double,
) -> *mut libc::c_char {
    let timestamp_str = get_kml_timestamp(timestamp);
    let latitude_str = dc_ftoa(latitude);
    let longitude_str = dc_ftoa(longitude);

    let ret = dc_mprintf(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <kml xmlns=\"http://www.opengis.net/kml/2.2\">\n\
         <Document>\n\
         <Placemark>\
         <Timestamp><when>%s</when></Timestamp>\
         <Point><coordinates>%s,%s</coordinates></Point>\
         </Placemark>\n\
         </Document>\n\
         </kml>\x00" as *const u8 as *const libc::c_char,
        timestamp_str,
        longitude_str, // reverse order!
        latitude_str,
    );

    free(latitude_str as *mut libc::c_void);
    free(longitude_str as *mut libc::c_void);
    free(timestamp_str as *mut libc::c_void);

    ret
}

pub unsafe fn dc_set_kml_sent_timestamp(context: &Context, chat_id: uint32_t, timestamp: i64) {
    let stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"UPDATE chats SET locations_last_sent=? WHERE id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
    sqlite3_bind_int(stmt, 2i32, chat_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

pub unsafe fn dc_set_msg_location_id(context: &Context, msg_id: uint32_t, location_id: uint32_t) {
    let stmt: *mut sqlite3_stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"UPDATE msgs SET location_id=? WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt, 1i32, location_id as sqlite3_int64);
    sqlite3_bind_int(stmt, 2i32, msg_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

pub unsafe fn dc_save_locations(
    context: &Context,
    chat_id: uint32_t,
    contact_id: uint32_t,
    locations: *const dc_array_t,
    independent: libc::c_int,
) -> uint32_t {
    let mut stmt_test: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut stmt_insert: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut newest_timestamp = 0;
    let mut newest_location_id: uint32_t = 0i32 as uint32_t;
    if !(chat_id <= 9i32 as libc::c_uint || locations.is_null()) {
        stmt_test = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"SELECT id FROM locations WHERE timestamp=? AND from_id=?\x00" as *const u8
                as *const libc::c_char,
        );
        stmt_insert = dc_sqlite3_prepare(
            context,
            &context.sql,
            b"INSERT INTO locations\
                  (timestamp, from_id, chat_id, latitude, longitude, accuracy, independent) \
                  VALUES (?,?,?,?,?,?,?);\x00" as *const u8 as *const libc::c_char,
        );
        let mut i = 0;
        while i < dc_array_get_cnt(locations) {
            let location: *mut dc_location_t =
                dc_array_get_ptr(locations, i as size_t) as *mut dc_location_t;
            sqlite3_reset(stmt_test);
            sqlite3_bind_int64(stmt_test, 1i32, (*location).timestamp as sqlite3_int64);
            sqlite3_bind_int(stmt_test, 2i32, contact_id as libc::c_int);
            if independent | sqlite3_step(stmt_test) != 100i32 {
                sqlite3_reset(stmt_insert);
                sqlite3_bind_int64(stmt_insert, 1i32, (*location).timestamp as sqlite3_int64);
                sqlite3_bind_int(stmt_insert, 2i32, contact_id as libc::c_int);
                sqlite3_bind_int(stmt_insert, 3i32, chat_id as libc::c_int);
                sqlite3_bind_double(stmt_insert, 4i32, (*location).latitude);
                sqlite3_bind_double(stmt_insert, 5i32, (*location).longitude);
                sqlite3_bind_double(stmt_insert, 6i32, (*location).accuracy);
                sqlite3_bind_double(stmt_insert, 7i32, independent as libc::c_double);
                sqlite3_step(stmt_insert);
            }
            if (*location).timestamp > newest_timestamp {
                newest_timestamp = (*location).timestamp;
                newest_location_id = dc_sqlite3_get_rowid2(
                    context,
                    &context.sql,
                    b"locations\x00" as *const u8 as *const libc::c_char,
                    b"timestamp\x00" as *const u8 as *const libc::c_char,
                    (*location).timestamp as uint64_t,
                    b"from_id\x00" as *const u8 as *const libc::c_char,
                    contact_id,
                )
            }
            i += 1
        }
    }
    sqlite3_finalize(stmt_test);
    sqlite3_finalize(stmt_insert);

    newest_location_id
}

pub unsafe fn dc_kml_parse(
    context: &Context,
    content: *const libc::c_char,
    content_bytes: size_t,
) -> *mut dc_kml_t {
    let mut kml: *mut dc_kml_t = calloc(1, ::std::mem::size_of::<dc_kml_t>()) as *mut dc_kml_t;
    let mut content_nullterminated: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut saxparser: dc_saxparser_t = dc_saxparser_t {
        starttag_cb: None,
        endtag_cb: None,
        text_cb: None,
        userdata: 0 as *mut libc::c_void,
    };

    if content_bytes > (1 * 1024 * 1024) {
        dc_log_warning(
            context,
            0,
            b"A kml-files with %i bytes is larger than reasonably expected.\x00" as *const u8
                as *const libc::c_char,
            content_bytes,
        );
    } else {
        content_nullterminated = dc_null_terminate(content, content_bytes as libc::c_int);
        if !content_nullterminated.is_null() {
            (*kml).locations = dc_array_new_typed(1, 100 as size_t);
            dc_saxparser_init(&mut saxparser, kml as *mut libc::c_void);
            dc_saxparser_set_tag_handler(
                &mut saxparser,
                Some(kml_starttag_cb),
                Some(kml_endtag_cb),
            );
            dc_saxparser_set_text_handler(&mut saxparser, Some(kml_text_cb));
            dc_saxparser_parse(&mut saxparser, content_nullterminated);
        }
    }

    free(content_nullterminated as *mut libc::c_void);

    kml
}

unsafe fn kml_text_cb(userdata: *mut libc::c_void, text: *const libc::c_char, _len: libc::c_int) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if 0 != (*kml).tag & (0x4 | 0x10) {
        let mut val: *mut libc::c_char = dc_strdup(text);
        dc_str_replace(
            &mut val,
            b"\n\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b"\r\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b"\t\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        dc_str_replace(
            &mut val,
            b" \x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        if 0 != (*kml).tag & 0x4 && strlen(val) >= 19 {
            // YYYY-MM-DDTHH:MM:SSZ
            // 0   4  7  10 13 16 19
            let val_r = as_str(val);
            match chrono::NaiveDateTime::parse_from_str(val_r, "%Y-%m-%dT%H:%M:%SZ") {
                Ok(res) => {
                    (*kml).curr.timestamp = res.timestamp();
                    if (*kml).curr.timestamp > time() {
                        (*kml).curr.timestamp = time();
                    }
                }
                Err(_err) => {
                    (*kml).curr.timestamp = time();
                }
            }
        } else if 0 != (*kml).tag & 0x10i32 {
            let mut comma: *mut libc::c_char = strchr(val, ',' as i32);
            if !comma.is_null() {
                let longitude: *mut libc::c_char = val;
                let latitude: *mut libc::c_char = comma.offset(1isize);
                *comma = 0i32 as libc::c_char;
                comma = strchr(latitude, ',' as i32);
                if !comma.is_null() {
                    *comma = 0i32 as libc::c_char
                }
                (*kml).curr.latitude = dc_atof(latitude);
                (*kml).curr.longitude = dc_atof(longitude)
            }
        }
        free(val as *mut libc::c_void);
    };
}

unsafe fn kml_endtag_cb(userdata: *mut libc::c_void, tag: *const libc::c_char) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if strcmp(tag, b"placemark\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if 0 != (*kml).tag & 0x1i32
            && 0 != (*kml).curr.timestamp
            && 0. != (*kml).curr.latitude
            && 0. != (*kml).curr.longitude
        {
            let location: *mut dc_location_t =
                calloc(1, ::std::mem::size_of::<dc_location_t>()) as *mut dc_location_t;
            *location = (*kml).curr;
            dc_array_add_ptr((*kml).locations, location as *mut libc::c_void);
        }
        (*kml).tag = 0i32
    };
}

/*******************************************************************************
 * parse kml-files
 ******************************************************************************/
unsafe fn kml_starttag_cb(
    userdata: *mut libc::c_void,
    tag: *const libc::c_char,
    attr: *mut *mut libc::c_char,
) {
    let mut kml: *mut dc_kml_t = userdata as *mut dc_kml_t;
    if strcmp(tag, b"document\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let addr: *const libc::c_char =
            dc_attr_find(attr, b"addr\x00" as *const u8 as *const libc::c_char);
        if !addr.is_null() {
            (*kml).addr = dc_strdup(addr)
        }
    } else if strcmp(tag, b"placemark\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*kml).tag = 0x1i32;
        (*kml).curr.timestamp = 0;
        (*kml).curr.latitude = 0i32 as libc::c_double;
        (*kml).curr.longitude = 0.0f64;
        (*kml).curr.accuracy = 0.0f64
    } else if strcmp(tag, b"timestamp\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x1i32
    {
        (*kml).tag = 0x1i32 | 0x2i32
    } else if strcmp(tag, b"when\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x2i32
    {
        (*kml).tag = 0x1i32 | 0x2i32 | 0x4i32
    } else if strcmp(tag, b"point\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x1i32
    {
        (*kml).tag = 0x1i32 | 0x8i32
    } else if strcmp(tag, b"coordinates\x00" as *const u8 as *const libc::c_char) == 0i32
        && 0 != (*kml).tag & 0x8i32
    {
        (*kml).tag = 0x1i32 | 0x8i32 | 0x10i32;
        let accuracy: *const libc::c_char =
            dc_attr_find(attr, b"accuracy\x00" as *const u8 as *const libc::c_char);
        if !accuracy.is_null() {
            (*kml).curr.accuracy = dc_atof(accuracy)
        }
    };
}

pub unsafe fn dc_kml_unref(kml: *mut dc_kml_t) {
    if kml.is_null() {
        return;
    }
    dc_array_unref((*kml).locations);
    free((*kml).addr as *mut libc::c_void);
    free(kml as *mut libc::c_void);
}

pub unsafe fn dc_job_do_DC_JOB_MAYBE_SEND_LOCATIONS(context: &Context, _job: *mut dc_job_t) {
    let stmt_chats: *mut sqlite3_stmt;
    let mut stmt_locations: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let now = time();
    let mut continue_streaming: libc::c_int = 1i32;
    dc_log_info(
        context,
        0i32,
        b" ----------------- MAYBE_SEND_LOCATIONS -------------- \x00" as *const u8
            as *const libc::c_char,
    );
    stmt_chats = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT id, locations_send_begin, locations_last_sent \
              FROM chats \
              WHERE locations_send_until>?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt_chats, 1i32, now as sqlite3_int64);
    while sqlite3_step(stmt_chats) == 100i32 {
        let chat_id: uint32_t = sqlite3_column_int(stmt_chats, 0i32) as uint32_t;
        let locations_send_begin = sqlite3_column_int64(stmt_chats, 1i32) as i64;
        let locations_last_sent = sqlite3_column_int64(stmt_chats, 2i32) as i64;
        continue_streaming = 1i32;
        // be a bit tolerant as the timer may not align exactly with time(NULL)
        if now - locations_last_sent < (60 - 3) {
            continue;
        }
        if stmt_locations.is_null() {
            stmt_locations = dc_sqlite3_prepare(
                context,
                &context.sql,
                b"SELECT id \
                  FROM locations \
                  WHERE from_id=? \
                  AND timestamp>=? \
                  AND timestamp>? \
                  AND independent=0 \
                  ORDER BY timestamp;\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            sqlite3_reset(stmt_locations);
        }
        sqlite3_bind_int(stmt_locations, 1i32, 1i32);
        sqlite3_bind_int64(stmt_locations, 2i32, locations_send_begin as sqlite3_int64);
        sqlite3_bind_int64(stmt_locations, 3i32, locations_last_sent as sqlite3_int64);
        // if there is no new location, there's nothing to send.
        // however, maybe we want to bypass this test eg. 15 minutes
        if sqlite3_step(stmt_locations) != 100i32 {
            continue;
        }
        // pending locations are attached automatically to every message,
        // so also to this empty text message.
        // DC_CMD_LOCATION is only needed to create a nicer subject.
        //
        // for optimisation and to avoid flooding the sending queue,
        // we could sending these messages only if we're really online.
        // the easiest way to determine this, is to check for an empty message queue.
        // (might not be 100%, however, as positions are sent combined later
        // and dc_set_location() is typically called periodically, this is ok)
        let mut msg: *mut dc_msg_t = dc_msg_new(context, 10i32);
        (*msg).hidden = 1i32;
        dc_param_set_int((*msg).param, 'S' as i32, 9i32);
        dc_send_msg(context, chat_id, msg);
        dc_msg_unref(msg);
    }
    if 0 != continue_streaming {
        schedule_MAYBE_SEND_LOCATIONS(context, 0x1i32);
    }
    sqlite3_finalize(stmt_chats);
    sqlite3_finalize(stmt_locations);
}

pub unsafe fn dc_job_do_DC_JOB_MAYBE_SEND_LOC_ENDED(context: &Context, job: &mut dc_job_t) {
    // this function is called when location-streaming _might_ have ended for a chat.
    // the function checks, if location-streaming is really ended;
    // if so, a device-message is added if not yet done.
    let chat_id: uint32_t = (*job).foreign_id;
    let locations_send_begin: i64;
    let locations_send_until: i64;
    let mut stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql,
        b"SELECT locations_send_begin, locations_send_until  FROM chats  WHERE id=?\x00"
            as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
    if !(sqlite3_step(stmt) != 100i32) {
        locations_send_begin = sqlite3_column_int64(stmt, 0i32) as i64;
        locations_send_until = sqlite3_column_int64(stmt, 1i32) as i64;
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        if !(locations_send_begin != 0 && time() <= locations_send_until) {
            // still streaming -
            // may happen as several calls to dc_send_locations_to_chat()
            // do not un-schedule pending DC_MAYBE_SEND_LOC_ENDED jobs
            if !(locations_send_begin == 0 && locations_send_until == 0) {
                // not streaming, device-message already sent
                stmt =
                    dc_sqlite3_prepare(
                        context,
                        &context.sql,
                        b"UPDATE chats    SET locations_send_begin=0, locations_send_until=0  WHERE id=?\x00"
                            as *const u8 as
                            *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
                sqlite3_step(stmt);
                let stock_str =
                    to_cstring(context.stock_system_msg(StockId::MsgLocationDisabled, "", "", 0));
                dc_add_device_msg(context, chat_id, stock_str.as_ptr());
                context.call_cb(
                    Event::CHAT_MODIFIED,
                    chat_id as uintptr_t,
                    0i32 as uintptr_t,
                );
            }
        }
    }
    sqlite3_finalize(stmt);
}
