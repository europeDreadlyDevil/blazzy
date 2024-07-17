use std::ffi::{OsStr, OsString};
use std::fs::{Metadata};
use std::os::windows::prelude::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Local};
use tokio::sync::mpsc::UnboundedSender;
use winapi::um::fileapi::{CreateFileW, FindFirstChangeNotificationW, FindNextChangeNotification, GetFileAttributesExW, OPEN_EXISTING, WIN32_FILE_ATTRIBUTE_DATA};
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::{FILE_FLAG_BACKUP_SEMANTICS, ReadDirectoryChangesW};
use winapi::um::winnt::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SECURITY, FILE_NOTIFY_CHANGE_SIZE, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, HANDLE};
use serde::Serialize;
use winapi::um::minwinbase::GetFileExInfoStandard;

pub struct Observer {
    path: Vec<u16>,
    handle: HANDLE,
    dir_handle: HANDLE,
    buffer: [u8; 8192],
    bytes_returned: u32,
}

impl Observer {
    pub async fn init(path: &str) -> Self {
        let path = OsString::from(path).encode_wide().chain(Some(0)).collect::<Vec<u16>>();

        let handle = unsafe {
            FindFirstChangeNotificationW(
                path.as_ptr(),
                1, // Recursive
                FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_DIR_NAME |
                    FILE_NOTIFY_CHANGE_ATTRIBUTES | FILE_NOTIFY_CHANGE_SIZE |
                    FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_SECURITY,
            )
        };

        if handle.is_null() {
            panic!("Failed to create change notification");
        }

        let dir_handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                null_mut()
            )
        };

        if dir_handle == INVALID_HANDLE_VALUE {
            panic!("Failed to open directory to read changes");
        }

        Self {
            path,
            handle,
            dir_handle,
            buffer: [0u8; 8192],
            bytes_returned: 0,
        }
    }

    pub fn run(self, sender: Arc<UnboundedSender<(PathBuf, Data)>>, with_logs: bool) {
        let mut buffer = self.buffer;
        let mut bytes_returned = self.bytes_returned;
        loop {
            unsafe {
                let result = WaitForSingleObject(self.handle, 1);
                if result == 0 { // WAIT_OBJECT_0
                    let success = ReadDirectoryChangesW(
                        self.dir_handle,
                        buffer.as_mut_ptr() as *mut _,
                        buffer.len() as u32,
                        1, // Recursive
                        FILE_NOTIFY_CHANGE_FILE_NAME |
                            FILE_NOTIFY_CHANGE_DIR_NAME |
                            FILE_NOTIFY_CHANGE_ATTRIBUTES |
                            FILE_NOTIFY_CHANGE_SIZE |
                            FILE_NOTIFY_CHANGE_LAST_WRITE |
                            FILE_NOTIFY_CHANGE_SECURITY,
                        &mut bytes_returned,
                        null_mut(),
                        None
                    );

                    if success == 0 {
                        panic!("Error reading changes from directory");
                    }

                    let mut offset = 0;
                    while offset < bytes_returned as usize {
                        let notify_info = &*(buffer.as_ptr().add(offset) as *const FILE_NOTIFY_INFORMATION);

                        let filename_wide: Vec<u16> = (0..(notify_info.FileNameLength / 2))
                            .map(|i| { *notify_info.FileName.as_ptr().add(i as usize) })
                            .collect();
                        let filename_os_str = OsString::from_wide(&filename_wide);
                        let filename = filename_os_str.to_string_lossy();

                        let action = match notify_info.Action {
                            1 => Action::Created,
                            2 => Action::Deleted,
                            3 => Action::Modified,
                            4 => Action::RenamedIn,
                            5 => Action::RenamedOut,
                            _ => Action::UnknownAction,
                        };

                        if with_logs { println!("{action:?}: {filename:?}"); }

                        let file_path = format!("C:\\{}", filename);
                        match Self::get_file_metadata(&file_path) {
                            Ok(metadata) => {
                                sender.send((PathBuf::from(file_path.to_string()), Data::new(action, Some(metadata)))).unwrap();
                            }
                            Err(e) => {
                                sender.send((PathBuf::from(file_path.to_string()), Data::new(action, None))).unwrap();
                            }
                        };



                        offset += notify_info.NextEntryOffset as usize;
                        if notify_info.NextEntryOffset == 0 {
                            break;
                        }
                    }

                    if FindNextChangeNotification(self.handle) == 0 {
                        panic!("Error when calling FindNextChangeNotification");
                    }
                } else if result == 0x102 { // WAIT_TIMEOUT
                    continue
                } else {
                    panic!("Error while waiting: {:?}", result);
                }
            }
        }
    }

    fn get_file_metadata(path: &str) -> std::io::Result<MetadataWrapper> {
        let path_wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
        unsafe {
            let mut file_info: WIN32_FILE_ATTRIBUTE_DATA = std::mem::zeroed();

            if GetFileAttributesExW(path_wide.as_ptr(), GetFileExInfoStandard, &mut file_info as *mut _ as *mut _) == 0 {
                return Err(std::io::Error::last_os_error());
            }

            let file_type = if file_info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                "directory"
            } else if file_info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                "symlink"
            } else {
                "file"
            }.to_string();

            let permissions = format!("{:o}", file_info.dwFileAttributes);

            let created = filetime_to_systemtime(((file_info.ftCreationTime.dwHighDateTime as u64) << 32) | file_info.ftCreationTime.dwLowDateTime as u64);
            let accessed = filetime_to_systemtime(((file_info.ftLastAccessTime.dwHighDateTime as u64) << 32) | file_info.ftLastAccessTime.dwLowDateTime as u64);
            let modified = filetime_to_systemtime(((file_info.ftLastWriteTime.dwHighDateTime as u64) << 32) | file_info.ftLastWriteTime.dwLowDateTime as u64);

            let created_dt: DateTime<Local> = DateTime::from(created);
            let accessed_dt: DateTime<Local> = DateTime::from(accessed);
            let modified_dt: DateTime<Local> = DateTime::from(modified);

            return Ok(MetadataWrapper {
                file_type,
                is_dir: file_info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0,
                is_file: file_info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0,
                is_symlink: file_info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0,
                len_in_bytes: ((file_info.nFileSizeHigh as u64) << 32) | file_info.nFileSizeLow as u64,
                permissions,
                modified: modified_dt.to_rfc3339(),
                accessed: accessed_dt.to_rfc3339(),
                created: created_dt.to_rfc3339(),
            });
        }
        fn filetime_to_systemtime(ft: u64) -> SystemTime {
            UNIX_EPOCH + Duration::from_nanos((ft - 116444736000000000) * 100)
        }
    }

}

#[derive(Serialize, Debug, Clone)]
struct MetadataWrapper {
    file_type: String,
    is_dir: bool,
    is_file: bool,
    is_symlink: bool,
    len_in_bytes: u64,
    permissions: String,
    modified: String,
    accessed: String,
    created: String,
}

impl From<&Metadata> for MetadataWrapper {
    fn from(metadata: &Metadata) -> Self {
        MetadataWrapper {
            file_type: format!("{:?}", metadata.file_type()),
            is_dir: metadata.is_dir(),
            is_file: metadata.is_file(),
            is_symlink: metadata.is_symlink(),
            len_in_bytes: metadata.len(),
            permissions: format!("{:?}", metadata.permissions()),
            modified: format!("{:?}", metadata.modified().unwrap()),
            accessed: format!("{:?}", metadata.accessed().unwrap()),
            created: format!("{:?}", metadata.created().unwrap())
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Data {
    action: Action,
    metadata: Option<MetadataWrapper>,
}

impl Data {
    pub fn new(action: Action, metadata: Option<MetadataWrapper>) -> Self {
        Self {
            action,
            metadata
        }
    }
}

#[derive(Debug, Serialize, Copy, Clone)]
enum Action {
    Created,
    Deleted,
    Modified,
    RenamedIn,
    RenamedOut,
    UnknownAction
}
