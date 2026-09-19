# MakeMKV output

- [Output examples](#output-examples)
  - [`makemkvcon backup`](#makemkvcon-backup)
  - [`makemkvcon info` (drives)](#makemkvcon-info-drives)
  - [`makemkvcon info` (content)](#makemkvcon-info-content)

## Output examples

### `makemkvcon backup`

Example output for `makemkvcon --debug --noscan --progress --robot backup <...>`.

- computer has three optical drives (/dev/sr0, /dev/sr1 and /dev/sr2)
- disabled the device scanning (DRV records do not show disc type and name)
  to avoid disturbing other drives since we don't need this information
- enabled debug messages
- enabled progress updates (PRGC, PRGT, PRGV)

<!-- markdownlint-disable MD013 -->
```TEXT
MSG:1005,0,1,"MakeMKV v1.18.4 linux(x64-release) started","%1 started","MakeMKV v1.18.4 linux(x64-release)"
MSG:1004,131072,1,"Debug logging enabled, log will be saved as file://<logfile>","Debug logging enabled, log will be saved as %1","file://<logfile>"
PRGT:5018,0,"Scanning CD-ROM devices"
PRGC:5018,0,"Scanning CD-ROM devices"
PRGV:0,0,65536
PRGV:0,0,65536
MSG:1003,32,3,"DEBUG: Code 0 at <value>:29393631","DEBUG: Code %1 at %2:%3","0","<value>","29393631"
PRGV:16384,0,65536
<...>
PRGV:65536,65536,65536
DRV:0,0,999,0,<device_name>,"","/dev/sr1"
DRV:1,0,999,0,<device_name>,"","/dev/sr2"
DRV:2,0,999,0,<device_name>,"","/dev/sr0"
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
PRGV:0,65536,65536
PRGV:0,0,65536
MSG:5072,131072,1,"Backing up disc into folder \"file://<isofile>\"","Backing up disc into folder \"%1\"","file://<isofile>"
PRGT:5047,0,"Copying all files"
PRGC:3102,0,"Processing title sets"
PRGV:0,0,65536
<...>
PRGV:65535,65534,65536
MSG:5070,128,0,"Backup done","Backup done"
MSG:5081,260,0,"Backup done.","Backup done."
```
<!-- markdownlint-enable MD013 -->

### `makemkvcon info` (drives)

Example output for `makemkvcon --robot --debug --progress=-stdout info disc:-1`.

- computer has three optical drives (/dev/sr0, /dev/sr1 and /dev/sr2)
  - drive 0 and 1 contain a DVD
    (indicated by content flag 'DVD' (1) -> 1)
  - drive 2 contains a BluRay
    (indicated by content flags 'AACS' (4) + 'BDSVM' (8) -> 12)
- execution time depends on
  - number of optical drives
  - whether a medium is inserted
  - whether the drives are busy
- enabled debug messages
- enabled progress updates (PRGC, PRGT, PRGV)
- does not list any content because we intentionally used a non-existing
  drive (only interested in drive- and media-related output)

<!-- markdownlint-disable MD013 -->
```TEXT
MSG:1005,0,1,"MakeMKV v1.18.4 linux(x64-release) started","%1 started","MakeMKV v1.18.4 linux(x64-release)"
MSG:1004,131072,1,"Debug logging enabled, log will be saved as file://<logfile>","Debug logging enabled, log will be saved as %1","file://<logfile>"
PRGT:5018,0,"Scanning CD-ROM devices"
PRGC:5018,0,"Scanning CD-ROM devices"
PRGV:0,0,65536
PRGV:0,0,65536
MSG:1003,32,3,"DEBUG: Code 0 at <value>:29393631","DEBUG: Code %1 at %2:%3","0","<value>","29393631"
PRGV:16384,0,65536
PRGV:16384,16384,65536
PRGV:65536,16384,65536
PRGV:65536,65536,65536
DRV:0,2,999,1,"<device_name>","<disc_name>","/dev/sr1"
DRV:1,2,999,1,"<device_name>","<disc_name>","/dev/sr2"
DRV:2,2,999,12,"<device_name>","<disc_name>","/dev/sr0"
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
PRGV:0,65536,65536
PRGV:0,0,65536
MSG:1003,32,3,"DEBUG: Code 3 at <value>:121265062","DEBUG: Code %1 at %2:%3","3",<value>,"121265062"
MSG:5010,0,0,"Failed to open disc","Failed to open disc"
TCOUNT:0
```
<!-- markdownlint-enable MD013 -->

### `makemkvcon info` (content)

<!-- markdownlint-disable MD013 -->
Example output for `makemkvcon --robot --noscan --messages=-stdout --progress=-stdout --debug --directio=true --minlength=0 info file:<isofile>`.
<!-- markdownlint-enable MD013 -->

- computer has three optical drives (/dev/sr0, /dev/sr1 and /dev/sr2)
- reading from disk image
- enabled debug messages
- enabled progress updates (PRGC, PRGT, PRGV)
- disable media scanning

<!-- markdownlint-disable MD013 -->
```TEXT
MSG:1005,0,1,"MakeMKV v1.18.4 linux(x64-release) started","%1 started","MakeMKV v1.18.4 linux(x64-release)"
MSG:1004,131072,1,"Debug logging enabled, log will be saved as file://<filename>","Debug logging enabled, log will be saved as %1","file://<filename>"
PRGT:5018,0,"Scanning CD-ROM devices"
PRGC:5018,0,"Scanning CD-ROM devices"
PRGV:0,0,65536
PRGV:0,0,65536
MSG:1003,32,3,"DEBUG: Code 0 at <code>:29393631","DEBUG: Code %1 at %2:%3","0","<code>","29393631"
PRGV:16384,0,65536
<...>
PRGV:65536,65536,65536
MSG:1003,32,3,"DEBUG: Code 0 at <code>:29393631","DEBUG: Code %1 at %2:%3","0","<code>","29393631"
DRV:0,0,999,0,"DVD+R-DL PLDS DVD-RW DH16AFSH DL31 8SSDX0F17036L1CB5800MGJ","","/dev/sr1"
DRV:1,0,999,0,"BD-RE ASUS SBW-06D5H-U E101 AFDL222859WL","","/dev/sr2"
DRV:2,0,999,0,"BD-RE ASUS BW-16D1HT 3.10 KL1OBDB4635","","/dev/sr0"
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
PRGV:0,65536,65536
PRGV:0,0,65536
PRGT:3100,0,"Opening DVD disc"
MSG:3007,0,0,"Using direct disc access mode","Using direct disc access mode"
PRGC:3102,0,"Processing title sets"
PRGV:0,0,65536
<...>
PRGV:61166,6284,65536
PRGC:3120,1,"Scanning contents"
PRGV:0,6284,65536
<...>
PRGV:65536,51171,65536
PRGC:3103,0,"Processing titles"
PRGV:0,51171,65536
MSG:3037,16777216,1,"Cells 1-1 were removed from title start","Cells 1-%1 were removed from title start","1"
MSG:3038,16777216,2,"Cells 3-6 were removed from title end","Cells %1-%2 were removed from title end","3","6"
MSG:3026,16777216,3,"Title #1 declared length is 0:00:00 while its real length is 0:00:00 - assuming fake title","Title #%1 declared length is %2 while its real length is %3 - assuming fake title","1","0:00:00","0:00:00"
MSG:3038,16777216,2,"Cells 3-3 were removed from title end","Cells %1-%2 were removed from title end","3","3"
MSG:3028,0,3,"Title #2 was added (2 cell(s), 0:00:13)","Title #%1 was added (%2 cell(s), %3)","2","2","0:00:13"
PRGV:8192,51171,65536
PRGV:8192,52069,65536
MSG:3038,16777216,2,"Cells 2-2 were removed from title end","Cells %1-%2 were removed from title end","2","2"
MSG:3028,16777216,3,"Title #3 was added (1 cell(s), 0:00:07)","Title #%1 was added (%2 cell(s), %3)","3","1","0:00:07"
MSG:3038,16777216,2,"Cells 2-2 were removed from title end","Cells %1-%2 were removed from title end","2","2"
MSG:3028,0,3,"Title #4 was added (1 cell(s), 0:00:35)","Title #%1 was added (%2 cell(s), %3)","4","1","0:00:35"
PRGV:16384,52069,65536
PRGV:16384,52967,65536
MSG:3038,16777216,2,"Cells 7-9 were removed from title end","Cells %1-%2 were removed from title end","7","9"
MSG:3028,0,3,"Title #5 was added (6 cell(s), 0:24:36)","Title #%1 was added (%2 cell(s), %3)","5","6","0:24:36"
PRGV:20480,52967,65536
PRGV:20480,53416,65536
MSG:3038,16777216,2,"Cells 7-9 were removed from title end","Cells %1-%2 were removed from title end","7","9"
MSG:3028,0,3,"Title #6 was added (6 cell(s), 0:24:32)","Title #%1 was added (%2 cell(s), %3)","6","6","0:24:32"
PRGV:24576,53416,65536
PRGV:24576,53865,65536
MSG:3038,16777216,2,"Cells 7-9 were removed from title end","Cells %1-%2 were removed from title end","7","9"
MSG:3028,0,3,"Title #7 was added (6 cell(s), 0:24:33)","Title #%1 was added (%2 cell(s), %3)","7","6","0:24:33"
PRGV:28672,53865,65536
PRGV:28672,54314,65536
MSG:3038,16777216,2,"Cells 7-9 were removed from title end","Cells %1-%2 were removed from title end","7","9"
MSG:3028,0,3,"Title #8 was added (6 cell(s), 0:24:38)","Title #%1 was added (%2 cell(s), %3)","8","6","0:24:38"
PRGV:32768,54314,65536
PRGV:32768,54762,65536
MSG:3038,16777216,2,"Cells 8-10 were removed from title end","Cells %1-%2 were removed from title end","8","10"
MSG:3028,0,3,"Title #9 was added (7 cell(s), 0:26:54)","Title #%1 was added (%2 cell(s), %3)","9","7","0:26:54"
PRGV:36864,54762,65536
PRGV:36864,55211,65536
MSG:3038,16777216,2,"Cells 2-4 were removed from title end","Cells %1-%2 were removed from title end","2","4"
MSG:3028,0,3,"Title #10 was added (1 cell(s), 0:01:42)","Title #%1 was added (%2 cell(s), %3)","10","1","0:01:42"
PRGV:40960,55211,65536
PRGV:40960,55660,65536
MSG:3038,16777216,2,"Cells 2-4 were removed from title end","Cells %1-%2 were removed from title end","2","4"
MSG:3028,0,3,"Title #11 was added (1 cell(s), 0:01:24)","Title #%1 was added (%2 cell(s), %3)","11","1","0:01:24"
PRGV:45056,55660,65536
PRGV:45056,56109,65536
MSG:3038,16777216,2,"Cells 2-4 were removed from title end","Cells %1-%2 were removed from title end","2","4"
MSG:3028,0,3,"Title #12 was added (1 cell(s), 0:00:37)","Title #%1 was added (%2 cell(s), %3)","12","1","0:00:37"
PRGV:49152,56109,65536
PRGV:49152,56558,65536
MSG:3038,16777216,2,"Cells 2-4 were removed from title end","Cells %1-%2 were removed from title end","2","4"
MSG:3028,0,3,"Title #13 was added (1 cell(s), 0:01:54)","Title #%1 was added (%2 cell(s), %3)","13","1","0:01:54"
PRGV:53248,56558,65536
PRGV:53248,57007,65536
MSG:3038,16777216,2,"Cells 2-4 were removed from title end","Cells %1-%2 were removed from title end","2","4"
MSG:3028,0,3,"Title #14 was added (1 cell(s), 0:01:37)","Title #%1 was added (%2 cell(s), %3)","14","1","0:01:37"
PRGC:3104,0,"Decrypting data"
PRGV:0,57007,65536
<...>
PRGV:65536,63740,65536
MSG:5011,0,0,"Operation successfully completed","Operation successfully completed"
TCOUNT:13
CINFO:1,6206,"DVD disc"
CINFO:2,0,"BETTERMAN_V1"
CINFO:30,0,"BETTERMAN_V1"
CINFO:31,6119,"<b>Source information</b><br>"
CINFO:32,0,"BETTERMAN_V1"
CINFO:33,0,"0"
TINFO:0,8,0,"2"
TINFO:0,9,0,"0:00:13"
TINFO:0,10,0,"3.7 MB"
TINFO:0,11,0,"3917824"
TINFO:0,24,0,"02"
TINFO:0,25,0,"1"
TINFO:0,26,0,"1-2"
TINFO:0,27,0,"A1_t00.mkv"
TINFO:0,30,0,"2 chapter(s) , 3.7 MB (A1)"
TINFO:0,31,6120,"<b>Title information</b><br>"
TINFO:0,33,0,"0"
TINFO:0,49,0,"A1"
SINFO:0,0,1,6201,"Video"
SINFO:0,0,5,0,"V_MPEG2"
SINFO:0,0,6,0,"Mpeg2"
SINFO:0,0,7,0,"Mpeg2"
SINFO:0,0,13,0,"5 Mb/s"
SINFO:0,0,19,0,"720x480"
SINFO:0,0,20,0,"4:3"
SINFO:0,0,21,0,"29.97 (30000/1001)"
SINFO:0,0,22,0,"0"
SINFO:0,0,30,0,"Mpeg2"
SINFO:0,0,31,6121,"<b>Track information</b><br>"
SINFO:0,0,33,0,"0"
SINFO:0,0,38,0,""
SINFO:0,0,42,5088,"( Lossless conversion )"
SINFO:0,1,1,6202,"Audio"
<...>
SINFO:0,1,42,5088,"( Lossless conversion )"
TINFO:1,8,0,"1"
<...>
TINFO:1,49,0,"A1"
SINFO:1,0,1,6201,"Video"
<...>
SINFO:1,1,42,5088,"( Lossless conversion )"
<...>
SINFO:12,1,42,5088,"( Lossless conversion )"
```
<!-- markdownlint-enable MD013 -->

There is no further output after the last SINFO record. The program simply
terminates without providing a success/failure indication.
