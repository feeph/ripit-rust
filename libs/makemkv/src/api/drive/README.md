# drive record (DRV)

The 'DRV' record identifies

- which optical drives have been found
- which internal id has been assigned
- which media has been identified

## Example

This is the output of a Linux system with 2 optical drives:

```TEXT
MSG:1005,0,1,"MakeMKV v1.18.1 linux(x64-release) started","%1 started","MakeMKV v1.18.1 linux(x64-release)"
DRV:0,0,999,0,"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ","","/dev/sr1"
DRV:1,0,999,0,"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635","","/dev/sr0"
DRV:2,256,999,0,"","",""
DRV:3,256,999,0,"","",""
DRV:4,256,999,0,"","",""
DRV:5,256,999,0,"","",""
DRV:6,256,999,0,"","",""
DRV:7,256,999,0,"","",""
DRV:8,256,999,0,"","",""
DRV:9,256,999,0,"","",""
DRV:10,256,999,0,"","",""
DRV:11,256,999,0,"","",""
DRV:12,256,999,0,"","",""
DRV:13,256,999,0,"","",""
DRV:14,256,999,0,"","",""
DRV:15,256,999,0,"","",""
```

## device name vs. device id

If there are multiple drives then MakeMKV may present them in a different
order compared to the operating systems. The OS-provided device names/drive
letters and the MakeMKV-provided, numerical device IDs do not necessarily
share the same ordering.

The "DRV" message tells us how MakeMKV identifies the devices internally:

```TEXT
DRV:1,0,999,0,"<...>","<...>","/dev/sr0"
    ^-- device id              ^-- device name or drive letter
```

Awareness of this different ordering is vitally important for the correct
usage of `makemkv backup` since this command requires the disc's id to be
provided, you can't use the device name or drive letter. If the incorrect
id is used the wrong medium is read.

```BASH
# show medium's content
# - supports "iso:", "file:", "disc:" and "dev:"
# - using the device name/drive letter is convenient
makemkvcon info --robot dev:/dev/sr0 <directory>

# create a backup of this medium
# - must use "disc:", can't use "dev:"
# - in case of a multi-drive setup this may read from the wrong drive
makemkvcon backup --robot disc:0 <directory>
```

## Design Decisions

### 'drive_status_num' vs. 'drive_status'

The field `drive_status` contains the parsed value. If MakeMKV introduces an
additional, unsupported state then this field is set to `None`.

If this happens the unparsed value can be retrieved from `drive_status_num`
and reported to the user.

### 'content_type_num' vs. 'content_type'

The field `content_type` contains the parsed value. If MakeMKV introduces an
additional, unsupported state then this field is set to `None`.

If this happens the unparsed value can be retrieved from `content_type_num`
and reported to the user.

## Compatibility

### Undocumented field 'device_name'

According to <https://www.makemkv.com/developers/usage.txt> the DRV record
consists of 6 fields:

  `DRV:<index>,<visible>,<enabled>,<flags>,<drive_name>,<disc_name>`

however makemkvcon (1.8.x) returns an additional field 'device_name':

  `DRV:<index>,<visible>,<enabled>,<flags>,<drive_name>,<disc_name>,<device_name>`

e.g.:

```TEXT
DRV:0,2,999,1,"BD-RE ASUS SBW-06D5H-U E101 AFDL222859WL","COWBOY_BEBOP_V4","E:"
DRV:0,0,999,0,"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635","","/dev/sr0"
```

**Decision:** Ignore this difference and require usage of makemkvcon (1.8.x).

- It is unlikely that someone is still using 1.7.x since the maintainer
  expects and encourages users to update frequently.
- There are other cases where usage.txt is wrong (CINFO, TINFO and SINFO
  are documented to have the same format but they don't).
