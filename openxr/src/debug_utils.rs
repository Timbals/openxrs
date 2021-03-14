use std::ffi::{c_void, CStr};
use std::ptr::null;
use std::sync::Arc;

use crate::*;

// TODO object debug annotation
// TODO session labels

#[derive(Clone)]
pub struct DebugUtils {
    inner: Arc<DebugUtilsInner>,
}

pub type DebugCallback = fn(&str) -> ();

unsafe extern "system" fn internal_callback(
    _message_severity: DebugUtilsMessageSeverityFlagsEXT,
    _message_type: DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const sys::DebugUtilsMessengerCallbackDataEXT,
    user_data: *mut c_void,
) -> sys::Bool32 {
    // TODO use a pointer to the DebugUtils instance instead of directly using the callback
    let callback = *(user_data as *mut DebugCallback);

    let message = CStr::from_ptr((*callback_data).message)
        .to_str()
        .unwrap_or_default();

    // TODO pass extra information along
    callback(message);

    sys::FALSE
}

impl DebugUtils {
    /// Creates a new `DebugUtils` and `Instance`.
    /// This function should be used as an alternative to the `create_instance` function in `Entry`.
    ///
    /// The created debug messenger that calls the supplied callback will be destroyed when
    /// the created `DebugUtils` gets dropped.
    /// That means even when the created `DebugUtils` is only used for debug messages it should not
    /// be dropped immediately
    /// (`let (_, instance) = DebugUtils::new(...).unwrap();` would drop the `DebugUtils` immediately).
    ///
    /// `required_extensions.ext_debug_utils` needs to be enabled.
    pub fn new(
        entry: &Entry,
        app_info: &ApplicationInfo,
        required_extensions: &ExtensionSet,
        layers: &[&str],
        mut callback: DebugCallback,
    ) -> Result<(Self, Instance)> {
        // the XR_EXT_debug_utils extension is required for this safe wrapper
        if !required_extensions.ext_debug_utils {
            return Err(sys::Result::ERROR_EXTENSION_NOT_PRESENT);
        }

        let mut debug_messenger_create_info =
            Self::populate_debug_messenger_create_info(&mut callback);

        // the callback is used during the instance creation because the real debug messenger
        // can only be created after the instance has been created
        let instance = entry.create_instance_internal(
            app_info,
            required_extensions,
            layers,
            &mut debug_messenger_create_info as *const _ as *const _,
        )?;

        // the debug messenger is responsible for all callbacks
        // that happen during instance creation/destruction
        let debug_messenger_handle =
            Self::create_debug_messenger(&instance, &debug_messenger_create_info)?;

        // TODO move callback into inner to ensure lifetime of the callback pointer
        let inner = Arc::new(DebugUtilsInner {
            instance: instance.clone(),
            debug_messenger_handle,
        });

        Ok((Self { inner }, instance))
    }

    fn create_debug_messenger(
        instance: &Instance,
        create_info: &sys::DebugUtilsMessengerCreateInfoEXT,
    ) -> Result<sys::DebugUtilsMessengerEXT> {
        let mut messenger: sys::DebugUtilsMessengerEXT = Default::default();

        unsafe {
            cvt((instance
                .exts()
                .ext_debug_utils
                .unwrap()
                .create_debug_utils_messenger)(
                instance.as_raw(),
                create_info,
                &mut messenger,
            ))?;
        }

        Ok(messenger)
    }

    /// Populates the create info structure for a debug messenger.
    /// All messages (of any severity and type) will be passed along to the messenger.
    ///
    /// # Safety
    ///
    /// A raw pointer to the callback is used which means the lifetime of the reference
    /// must be as long as the lifetime of the debug messenger that gets created.
    fn populate_debug_messenger_create_info(
        callback: &mut DebugCallback,
    ) -> sys::DebugUtilsMessengerCreateInfoEXT {
        sys::DebugUtilsMessengerCreateInfoEXT {
            ty: sys::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            next: null(),
            message_severities: DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | DebugUtilsMessageSeverityFlagsEXT::INFO
                | DebugUtilsMessageSeverityFlagsEXT::WARNING
                | DebugUtilsMessageSeverityFlagsEXT::ERROR,
            message_types: DebugUtilsMessageTypeFlagsEXT::GENERAL
                | DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | DebugUtilsMessageTypeFlagsEXT::CONFORMANCE,
            user_callback: Some(internal_callback),
            user_data: callback as *mut _ as *mut c_void,
        }
    }
}

struct DebugUtilsInner {
    instance: Instance,
    debug_messenger_handle: sys::DebugUtilsMessengerEXT,
}

impl Drop for DebugUtilsInner {
    fn drop(&mut self) {
        println!("destroy debug utils");
        // TODO not dropped if new call destructures into just the instance
        unsafe {
            if let Some(extension) = self.instance.exts().ext_debug_utils {
                (extension.destroy_debug_utils_messenger)(self.debug_messenger_handle);
            }
        }
    }
}
