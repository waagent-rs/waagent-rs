use reqwest::Client;
use waagent_core::agent::{initialize_agent, run_heartbeat_loop};

// Windows service support
#[cfg(windows)]
use windows_service::{
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType},
    service_dispatcher,
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
};
#[cfg(windows)]
const SERVICE_NAME: &str = "waagent-rs-poc";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(windows)]
#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|arg| arg == "--service") {
        service_dispatcher::start(SERVICE_NAME, service_main as extern "system" fn(u32, *mut *mut u16)).unwrap();
        Ok(())
    } else {
        main_async().await
    }
}

#[cfg(not(windows))]
#[tokio::main]
async fn main() -> Result<()> {
    main_async().await
}
#[cfg(windows)]
extern "system" fn service_main(_argc: u32, _argv: *mut *mut u16) {
    // Register service control handler
    let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop | ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    }).unwrap();

    // Report running status
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
    process_id: Some(std::process::id()),
    });

    // Run main logic
    tokio::runtime::Runtime::new().unwrap().block_on(main_async()).unwrap();

    // Report stopped status
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
    process_id: Some(std::process::id()),
    });
}

async fn main_async() -> Result<()> {
    let client = Client::new();
    
    // Initialize agent and perform startup tasks
    let goal_state = initialize_agent(&client).await?;
    
    println!("\nStarting continuous heartbeat loop (send SIGINT/Ctrl+C to stop)...");
    
    // Run continuous heartbeat loop
    run_heartbeat_loop(&client, &goal_state).await?;
    
    Ok(())
}
