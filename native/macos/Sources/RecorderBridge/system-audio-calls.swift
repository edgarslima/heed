import CoreAudio
import Foundation

/// The Core Audio tap, aggregate-device, and IOProc entry points that system capture uses.
///
/// `live` forwards each call to the system without branching; every decision lives in
/// `SystemCapture`, so tests substitute these closures and exercise it without hardware.
struct SystemAudioCalls {
    var defaultOutputDevice: (inout AudioObjectID) -> OSStatus
    var deviceUID: (AudioObjectID, inout String) -> OSStatus
    var createTap: (CATapDescription, inout AudioObjectID) -> OSStatus
    var tapFormat: (AudioObjectID, inout AudioStreamBasicDescription) -> OSStatus
    var createAggregate: ([String: Any], inout AudioObjectID) -> OSStatus
    var nominalSampleRate: (AudioObjectID, inout Double) -> OSStatus
    var bufferFrameSize: (AudioObjectID, inout UInt32) -> OSStatus
    var createIOProc:
        (AudioObjectID, @escaping AudioDeviceIOProc, UnsafeMutableRawPointer, inout AudioDeviceIOProcID?) -> OSStatus
    var start: (AudioObjectID, AudioDeviceIOProcID) -> OSStatus
    var stop: (AudioObjectID, AudioDeviceIOProcID) -> OSStatus
    var destroyIOProc: (AudioObjectID, AudioDeviceIOProcID) -> OSStatus
    var destroyAggregate: (AudioObjectID) -> OSStatus
    var destroyTap: (AudioObjectID) -> OSStatus
    var hostClock: () -> HostClock

    static let live = SystemAudioCalls(
        defaultOutputDevice: { device in
            readProperty(AudioObjectID(kAudioObjectSystemObject), kAudioHardwarePropertyDefaultOutputDevice, &device)
        },
        deviceUID: { device, uid in
            var value: Unmanaged<CFString>?
            let status = readProperty(device, kAudioDevicePropertyDeviceUID, &value)
            // Core Audio returns CFString properties retained; the caller owns the reference.
            uid = value?.takeRetainedValue() as String? ?? ""
            return status
        },
        createTap: { AudioHardwareCreateProcessTap($0, &$1) },
        tapFormat: { readProperty($0, kAudioTapPropertyFormat, &$1) },
        createAggregate: { AudioHardwareCreateAggregateDevice($0 as CFDictionary, &$1) },
        nominalSampleRate: { readProperty($0, kAudioDevicePropertyNominalSampleRate, &$1) },
        bufferFrameSize: { readProperty($0, kAudioDevicePropertyBufferFrameSize, &$1) },
        createIOProc: { AudioDeviceCreateIOProcID($0, $1, $2, &$3) },
        start: { AudioDeviceStart($0, $1) },
        stop: { AudioDeviceStop($0, $1) },
        destroyIOProc: { AudioDeviceDestroyIOProcID($0, $1) },
        destroyAggregate: { AudioHardwareDestroyAggregateDevice($0) },
        destroyTap: { AudioHardwareDestroyProcessTap($0) },
        hostClock: { HostClock.system() })
}

/// Reads a global property whose size must match `Value` exactly.
private func readProperty<Value>(_ object: AudioObjectID, _ selector: AudioObjectPropertySelector, _ value: inout Value)
    -> OSStatus
{
    var address = AudioObjectPropertyAddress(
        mSelector: selector, mScope: kAudioObjectPropertyScopeGlobal, mElement: kAudioObjectPropertyElementMain)
    var size = UInt32(MemoryLayout<Value>.size)
    let status = withUnsafeMutablePointer(to: &value) { AudioObjectGetPropertyData(object, &address, 0, nil, &size, $0) }
    guard status == noErr else { return status }
    return size == UInt32(MemoryLayout<Value>.size) ? noErr : OSStatus(kAudioHardwareBadPropertySizeError)
}
