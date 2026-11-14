import AVFoundation
import AudioToolbox
import CoreAudio

/// Microphone access as the bridge reports it.
enum MicrophoneAccess: UInt32 {
    case notDetermined = 0
    case denied = 1
    case granted = 2
}

/// The Core Audio and TCC entry points that capture uses.
///
/// `live` forwards each call to the system without branching; every decision lives in
/// `MicrophoneCapture`, so tests substitute these closures and exercise it without hardware.
struct CoreAudioCalls {
    var defaultInputDevice: (UnsafeMutablePointer<AudioObjectID>) -> OSStatus
    var newInputUnit: (UnsafeMutablePointer<AudioUnit?>) -> OSStatus
    var setProperty:
        (AudioUnit, AudioUnitPropertyID, AudioUnitScope, AudioUnitElement, UnsafeRawPointer, UInt32) ->
            OSStatus
    var getProperty:
        (
            AudioUnit, AudioUnitPropertyID, AudioUnitScope, AudioUnitElement, UnsafeMutableRawPointer,
            UnsafeMutablePointer<UInt32>
        ) -> OSStatus
    var initialize: (AudioUnit) -> OSStatus
    var start: (AudioUnit) -> OSStatus
    var stop: (AudioUnit) -> OSStatus
    var uninitialize: (AudioUnit) -> OSStatus
    var dispose: (AudioUnit) -> OSStatus
    var render:
        (
            AudioUnit, UnsafeMutablePointer<AudioUnitRenderActionFlags>, UnsafePointer<AudioTimeStamp>,
            UInt32, UInt32, UnsafeMutablePointer<AudioBufferList>
        ) -> OSStatus
    var microphoneAccess: () -> AVAuthorizationStatus
    var requestMicrophoneAccess: (@escaping @Sendable (Bool) -> Void) -> Void
    var hostClock: () -> HostClock

    static let live = CoreAudioCalls(
        defaultInputDevice: { device in
            var address = AudioObjectPropertyAddress(
                mSelector: kAudioHardwarePropertyDefaultInputDevice,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain)
            var size = UInt32(MemoryLayout<AudioObjectID>.size)
            return AudioObjectGetPropertyData(
                AudioObjectID(kAudioObjectSystemObject), &address, 0, nil, &size, device)
        },
        newInputUnit: { unit in
            var description = AudioComponentDescription(
                componentType: kAudioUnitType_Output,
                componentSubType: kAudioUnitSubType_HALOutput,
                componentManufacturer: kAudioUnitManufacturer_Apple,
                componentFlags: 0,
                componentFlagsMask: 0)
            guard let component = AudioComponentFindNext(nil, &description) else {
                return OSStatus(kAudioHardwareUnspecifiedError)
            }
            return AudioComponentInstanceNew(component, unit)
        },
        setProperty: { AudioUnitSetProperty($0, $1, $2, $3, $4, $5) },
        getProperty: { AudioUnitGetProperty($0, $1, $2, $3, $4, $5) },
        initialize: { AudioUnitInitialize($0) },
        start: { AudioOutputUnitStart($0) },
        stop: { AudioOutputUnitStop($0) },
        uninitialize: { AudioUnitUninitialize($0) },
        dispose: { AudioComponentInstanceDispose($0) },
        render: { AudioUnitRender($0, $1, $2, $3, $4, $5) },
        microphoneAccess: { AVCaptureDevice.authorizationStatus(for: .audio) },
        requestMicrophoneAccess: { AVCaptureDevice.requestAccess(for: .audio, completionHandler: $0) },
        hostClock: { HostClock.system() })
}

/// Converts `mach_absolute_time` ticks to nanoseconds.
struct HostClock: Equatable {
    let numerator: UInt32
    let denominator: UInt32

    static func system() -> HostClock {
        var info = mach_timebase_info_data_t()
        guard mach_timebase_info(&info) == KERN_SUCCESS, info.denom != 0 else {
            return HostClock(numerator: 1, denominator: 1)
        }
        return HostClock(numerator: info.numer, denominator: info.denom)
    }

    /// Exact for any tick count; saturates instead of overflowing.
    func nanoseconds(_ ticks: UInt64) -> UInt64 {
        let denominator = UInt64(max(self.denominator, 1))
        let product = ticks.multipliedFullWidth(by: UInt64(numerator))
        guard product.high < denominator else { return UInt64.max }
        return denominator.dividingFullWidth(product).quotient
    }
}
