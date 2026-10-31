use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, TCP_TABLE_OWNER_PID_ALL,
};
use windows_sys::Win32::Networking::WinSock::AF_INET6;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetProcessIoCounters, IO_COUNTERS,
};
use std::ffi::c_void;
use std::mem::zeroed;

#[test]
fn test_get_process_io_counters() {
    unsafe {
        let handle = GetCurrentProcess();
        let mut counters: IO_COUNTERS = zeroed();
        let res = GetProcessIoCounters(handle, &mut counters);
        assert_ne!(res, 0, "GetProcessIoCounters should succeed on current process");
        println!(
            "Process IO: ReadTransfer={} B, WriteTransfer={} B",
            counters.ReadTransferCount, counters.WriteTransferCount
        );
    }
}

#[test]
fn test_tcp6_table_query() {
    unsafe {
        let mut size = 65536u32;
        let mut buffer: Vec<u8> = vec![0; size as usize];
        let res = GetExtendedTcpTable(
            buffer.as_mut_ptr() as *mut c_void,
            &mut size,
            0,
            AF_INET6 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
        println!("TCP6 query result code: {}, size: {}", res, size);
    }
}
