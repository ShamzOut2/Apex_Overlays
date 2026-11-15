//! Le Mans Ultimate's built-in shared memory ("LMU_Data"): a byte-for-byte mirror of the
//! layout in the game's `Support\SharedMemoryInterface` header (rFactor 2's InternalsPlugin
//! structures, packed to 4 bytes). Field names are kept as in the game's header so they can be
//! checked against it. The layout follows the MIT-licensed pyLMUSharedMemory project's mapping
//! (see THIRD_PARTY_NOTICES.md); the tests pin the sizes and offsets.
#![allow(non_snake_case, dead_code)]

pub const MAP_NAME: &str = "LMU_Data";
pub const MAX_VEHICLES: usize = 104;
const PATH: usize = 260;

#[repr(C, packed(4))]
#[derive(Clone, Copy, Default)]
pub struct Vect3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct Wheel {
    pub mSuspensionDeflection: f64,
    pub mRideHeight: f64,
    pub mSuspForce: f64,
    pub mBrakeTemp: f64,
    pub mBrakePressure: f64,
    pub mRotation: f64,
    pub mLateralPatchVel: f64,
    pub mLongitudinalPatchVel: f64,
    pub mLateralGroundVel: f64,
    pub mLongitudinalGroundVel: f64,
    pub mCamber: f64,
    pub mLateralForce: f64,
    pub mLongitudinalForce: f64,
    pub mTireLoad: f64,
    pub mGripFract: f64,
    pub mPressure: f64,
    pub mTemperature: [f64; 3],
    pub mWear: f64,
    pub mTerrainName: [u8; 16],
    pub mSurfaceType: u8,
    pub mFlat: u8,
    pub mDetached: u8,
    pub mStaticUndeflectedRadius: u8,
    pub mVerticalTireDeflection: f64,
    pub mWheelYLocation: f64,
    pub mToe: f64,
    pub mTireCarcassTemperature: f64,
    pub mTireInnerLayerTemperature: [f64; 3],
    pub mOptimalTemp: f32,
    pub mCompoundIndex: u8,
    /// 0 soft, 1 medium, 2 hard, 3 wet.
    pub mCompoundType: u8,
    pub mExpansion: [u8; 18],
}

/// TelemInfoV01: one car's physics, at up to 100 Hz.
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct VehicleTelemetry {
    pub mID: i32,
    pub mDeltaTime: f64,
    pub mElapsedTime: f64,
    pub mLapNumber: i32,
    pub mLapStartET: f64,
    pub mVehicleName: [u8; 64],
    pub mTrackName: [u8; 64],
    pub mPos: Vect3,
    pub mLocalVel: Vect3,
    pub mLocalAccel: Vect3,
    pub mOri: [Vect3; 3],
    pub mLocalRot: Vect3,
    pub mLocalRotAccel: Vect3,
    pub mGear: i32,
    pub mEngineRPM: f64,
    pub mEngineWaterTemp: f64,
    pub mEngineOilTemp: f64,
    pub mClutchRPM: f64,
    pub mUnfilteredThrottle: f64,
    pub mUnfilteredBrake: f64,
    pub mUnfilteredSteering: f64,
    pub mUnfilteredClutch: f64,
    pub mFilteredThrottle: f64,
    pub mFilteredBrake: f64,
    pub mFilteredSteering: f64,
    pub mFilteredClutch: f64,
    pub mSteeringShaftTorque: f64,
    pub mFront3rdDeflection: f64,
    pub mRear3rdDeflection: f64,
    pub mFrontWingHeight: f64,
    pub mFrontRideHeight: f64,
    pub mRearRideHeight: f64,
    pub mDrag: f64,
    pub mFrontDownforce: f64,
    pub mRearDownforce: f64,
    pub mFuel: f64,
    pub mEngineMaxRPM: f64,
    pub mScheduledStops: u8,
    pub mOverheating: u8,
    pub mDetached: u8,
    pub mHeadlights: u8,
    pub mDentSeverity: [u8; 8],
    pub mLastImpactET: f64,
    pub mLastImpactMagnitude: f64,
    pub mLastImpactPos: Vect3,
    pub mEngineTorque: f64,
    pub mCurrentSector: i32,
    pub mSpeedLimiter: u8,
    pub mMaxGears: u8,
    pub mFrontTireCompoundIndex: u8,
    pub mRearTireCompoundIndex: u8,
    pub mFuelCapacity: f64,
    pub mFrontFlapActivated: u8,
    pub mRearFlapActivated: u8,
    pub mRearFlapLegalStatus: u8,
    pub mIgnitionStarter: u8,
    pub mFrontTireCompoundName: [u8; 18],
    pub mRearTireCompoundName: [u8; 18],
    pub mSpeedLimiterAvailable: u8,
    pub mAntiStallActivated: u8,
    pub mUnused: [u8; 2],
    pub mVisualSteeringWheelRange: f32,
    pub mRearBrakeBias: f64,
    pub mTurboBoostPressure: f64,
    pub mPhysicsToGraphicsOffset: [f32; 3],
    /// Degrees, lock to lock.
    pub mPhysicalSteeringWheelRange: f32,
    pub mDeltaBest: f64,
    pub mBatteryChargeFraction: f64,
    pub mElectricBoostMotorTorque: f64,
    pub mElectricBoostMotorRPM: f64,
    pub mElectricBoostMotorTemperature: f64,
    pub mElectricBoostWaterTemperature: f64,
    pub mElectricBoostMotorState: u8,
    pub mLapInvalidated: u8,
    pub mABSActive: u8,
    pub mTCActive: u8,
    pub mSpeedLimiterActive: u8,
    pub mWiperState: u8,
    pub mTC: u8,
    pub mTCMax: u8,
    pub mTCSlip: u8,
    pub mTCSlipMax: u8,
    pub mTCCut: u8,
    pub mTCCutMax: u8,
    pub mABS: u8,
    pub mABSMax: u8,
    pub mMotorMap: u8,
    pub mMotorMapMax: u8,
    pub mMigration: u8,
    pub mMigrationMax: u8,
    pub mFrontAntiSway: u8,
    pub mFrontAntiSwayMax: u8,
    pub mRearAntiSway: u8,
    pub mRearAntiSwayMax: u8,
    pub mLiftAndCoastProgress: u8,
    pub mTrackLimitsSteps: u8,
    pub mRegen: f32,
    pub mStateOfCharge: f32,
    pub mVirtualEnergy: f32,
    pub mTimeGapCarAhead: f32,
    pub mTimeGapCarBehind: f32,
    pub mTimeGapPlaceAhead: f32,
    pub mTimeGapPlaceBehind: f32,
    pub mVehicleModel: [u8; 30],
    pub mVehicleClass: u8,
    pub mVehicleChampionship: u8,
    pub mExpansion: [u8; 20],
    pub mWheels: [Wheel; 4],
}

/// VehicleScoringInfoV01: one car's timing and standing, at about 5 Hz.
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct VehicleScoring {
    pub mID: i32,
    pub mDriverName: [u8; 32],
    pub mVehicleName: [u8; 64],
    pub mTotalLaps: i16,
    /// 0 = sector 3, 1 = sector 1, 2 = sector 2.
    pub mSector: i8,
    /// 0 none, 1 finished, 2 DNF, 3 DQ.
    pub mFinishStatus: i8,
    pub mLapDist: f64,
    pub mPathLateral: f64,
    pub mTrackEdge: f64,
    pub mBestSector1: f64,
    pub mBestSector2: f64,
    pub mBestLapTime: f64,
    pub mLastSector1: f64,
    pub mLastSector2: f64,
    pub mLastLapTime: f64,
    pub mCurSector1: f64,
    pub mCurSector2: f64,
    pub mNumPitstops: i16,
    pub mNumPenalties: i16,
    pub mIsPlayer: u8,
    /// -1 nobody, 0 local player, 1 local AI, 2 remote, 3 replay.
    pub mControl: i8,
    pub mInPits: u8,
    pub mPlace: u8,
    pub mVehicleClass: [u8; 32],
    pub mTimeBehindNext: f64,
    pub mLapsBehindNext: i32,
    pub mTimeBehindLeader: f64,
    pub mLapsBehindLeader: i32,
    pub mLapStartET: f64,
    pub mPos: Vect3,
    pub mLocalVel: Vect3,
    pub mLocalAccel: Vect3,
    pub mOri: [Vect3; 3],
    pub mLocalRot: Vect3,
    pub mLocalRotAccel: Vect3,
    pub mHeadlights: u8,
    /// 0 none, 1 request, 2 entering, 3 stopped, 4 exiting.
    pub mPitState: u8,
    pub mServerScored: u8,
    pub mIndividualPhase: u8,
    pub mQualification: i32,
    pub mTimeIntoLap: f64,
    pub mEstimatedLapTime: f64,
    pub mPitGroup: [u8; 24],
    /// 0 green, 6 blue.
    pub mFlag: u8,
    pub mUnderYellow: u8,
    pub mCountLapFlag: u8,
    pub mInGarageStall: u8,
    pub mUpgradePack: [u8; 16],
    pub mPitLapDist: f32,
    pub mBestLapSector1: f32,
    pub mBestLapSector2: f32,
    pub mSteamID: u64,
    pub mVehFilename: [u8; 32],
    pub mAttackMode: i16,
    pub mFuelFraction: u8,
    pub mDRSState: u8,
    pub mExpansion: [u8; 4],
}

/// ScoringInfoV01: the session.
#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct ScoringInfo {
    pub mTrackName: [u8; 64],
    /// 0 test day, 1-4 practice, 5-8 qualifying, 9 warm-up, 10-13 race.
    pub mSession: i32,
    pub mCurrentET: f64,
    pub mEndET: f64,
    pub mMaxLaps: i32,
    pub mLapDist: f64,
    pub mResultsStreamPointer: [u8; 8],
    pub mNumVehicles: i32,
    /// 0 before the session, 1 recon laps, 2 grid walk, 3 formation lap, 4 start countdown,
    /// 5 green, 6 full-course yellow / safety car, 7 stopped, 8 over, 9 paused.
    pub mGamePhase: u8,
    /// -1 invalid, 0 none, 1 pending, 2 pits closed, 3 pit lead lap, 4 pits open, 5 last lap, 6 resume.
    pub mYellowFlagState: i8,
    pub mSectorFlag: [u8; 3],
    pub mStartLight: u8,
    pub mNumRedLights: u8,
    pub mInRealtime: u8,
    pub mPlayerName: [u8; 32],
    pub mPlrFileName: [u8; 64],
    pub mDarkCloud: f64,
    pub mRaining: f64,
    pub mAmbientTemp: f64,
    pub mTrackTemp: f64,
    pub mWind: Vect3,
    pub mMinPathWetness: f64,
    pub mMaxPathWetness: f64,
    pub mGameMode: u8,
    pub mIsPasswordProtected: u8,
    pub mServerPort: u16,
    pub mServerPublicIP: u32,
    pub mMaxPlayers: i32,
    pub mServerName: [u8; 32],
    pub mStartET: f32,
    pub mAvgPathWetness: f64,
    pub mSessionTimeRemaining: f32,
    pub mTimeOfDay: f32,
    pub mIsFixedSetup: u8,
    /// 0 green, 1 low, 2 medium, 3 high, 4 saturated.
    pub mTrackGripLevel: u8,
    /// 0 clear ... 4 overcast, 5+ cloud with rain.
    pub mCloudCoverage: u8,
    pub mTrackLimitsStepsPerPenalty: u8,
    pub mTrackLimitsStepsPerPoint: u8,
    pub mExpansion: [u8; 187],
    pub mVehiclePointer: [u8; 8],
}

#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct ApplicationState {
    pub mAppWindow: u64,
    pub mWidth: u32,
    pub mHeight: u32,
    pub mRefreshRate: u32,
    pub mWindowed: u32,
    pub mOptionsLocation: u8,
    pub mOptionsPage: [u8; 31],
    pub mExpansion: [u8; 204],
}

#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct Events {
    pub SME_ENTER: u32,
    pub SME_EXIT: u32,
    pub SME_STARTUP: u32,
    pub SME_SHUTDOWN: u32,
    pub SME_LOAD: u32,
    pub SME_UNLOAD: u32,
    pub SME_START_SESSION: u32,
    pub SME_END_SESSION: u32,
    pub SME_ENTER_REALTIME: u32,
    pub SME_EXIT_REALTIME: u32,
    pub SME_UPDATE_SCORING: u32,
    pub SME_UPDATE_TELEMETRY: u32,
    pub SME_INIT_APPLICATION: u32,
    pub SME_UNINIT_APPLICATION: u32,
    pub SME_SET_ENVIRONMENT: u32,
    pub SME_FFB: u32,
}

#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct Generic {
    pub events: Events,
    pub gameVersion: i32,
    pub FFBTorque: f32,
    pub appInfo: ApplicationState,
}

#[repr(C, packed(4))]
#[derive(Clone, Copy)]
pub struct Paths {
    pub userData: [u8; PATH],
    pub customVariables: [u8; PATH],
    pub stewardResults: [u8; PATH],
    pub playerProfile: [u8; PATH],
    pub pluginsFolder: [u8; PATH],
}

#[repr(C, packed(4))]
pub struct ScoringData {
    pub scoringInfo: ScoringInfo,
    pub scoringStreamSize: [u8; 12],
    pub vehScoringInfo: [VehicleScoring; MAX_VEHICLES],
    pub scoringStream: [u8; 65536],
}

#[repr(C, packed(4))]
pub struct TelemetryData {
    pub activeVehicles: u8,
    pub playerVehicleIdx: u8,
    pub playerHasVehicle: u8,
    pub telemInfo: [VehicleTelemetry; MAX_VEHICLES],
}

/// The whole mapped file.
#[repr(C, packed(4))]
pub struct Layout {
    pub generic: Generic,
    pub paths: Paths,
    pub scoring: ScoringData,
    pub telemetry: TelemetryData,
}

/// A fixed-size text field as a string (up to the first NUL, Latin-1 / UTF-8 tolerant).
pub fn text(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let s = &b[..end];
    match std::str::from_utf8(s) {
        Ok(t) => t.trim().to_string(),
        // The game writes Windows-1252; treat bytes as Latin-1 when not UTF-8.
        Err(_) => s.iter().map(|&c| c as char).collect::<String>().trim().to_string(),
    }
}

/// Zeroed values for tests and as the starting point before the first read.
macro_rules! zeroed {
    ($($t:ty),*) => {$(
        impl $t {
            pub fn zeroed() -> Self {
                // SAFETY: plain old data made of integers, floats and byte arrays: all zeroes is valid.
                unsafe { std::mem::zeroed() }
            }
        }
    )*};
}
zeroed!(VehicleTelemetry, VehicleScoring, ScoringInfo, Generic);

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    /// Sizes and offsets computed from the documented layout with Python's ctypes (pack = 4),
    /// which lays out structures as the game's compiler does.
    #[test]
    fn layout_matches_the_game() {
        assert_eq!(size_of::<Vect3>(), 24);
        assert_eq!(size_of::<Wheel>(), 260);
        assert_eq!(size_of::<VehicleTelemetry>(), 1888);
        assert_eq!(size_of::<VehicleScoring>(), 584);
        assert_eq!(size_of::<ScoringInfo>(), 548);
        assert_eq!(size_of::<ApplicationState>(), 260);
        assert_eq!(size_of::<ScoringData>(), 126_832);
        assert_eq!(size_of::<TelemetryData>(), 196_356);
        assert_eq!(size_of::<Paths>(), 1300);
        assert_eq!(size_of::<Events>(), 64);
        assert_eq!(size_of::<Generic>(), 332);
        assert_eq!(size_of::<Layout>(), 324_820);

        assert_eq!(
            [offset_of!(Layout, generic), offset_of!(Layout, paths), offset_of!(Layout, scoring), offset_of!(Layout, telemetry)],
            [0, 332, 1632, 128_464]
        );
        assert_eq!([offset_of!(ScoringData, scoringInfo), offset_of!(ScoringData, vehScoringInfo), offset_of!(ScoringData, scoringStream)], [0, 560, 61_296]);
        assert_eq!(offset_of!(TelemetryData, telemInfo), 4);
        assert_eq!(
            [
                offset_of!(ScoringInfo, mSession),
                offset_of!(ScoringInfo, mCurrentET),
                offset_of!(ScoringInfo, mNumVehicles),
                offset_of!(ScoringInfo, mGamePhase),
                offset_of!(ScoringInfo, mPlayerName),
                offset_of!(ScoringInfo, mDarkCloud),
                offset_of!(ScoringInfo, mWind),
                offset_of!(ScoringInfo, mServerPort),
                offset_of!(ScoringInfo, mServerName),
                offset_of!(ScoringInfo, mAvgPathWetness),
                offset_of!(ScoringInfo, mTimeOfDay),
                offset_of!(ScoringInfo, mExpansion),
            ],
            [64, 68, 104, 108, 116, 212, 244, 286, 296, 332, 344, 353]
        );
        assert_eq!(
            [
                offset_of!(VehicleScoring, mTotalLaps),
                offset_of!(VehicleScoring, mLapDist),
                offset_of!(VehicleScoring, mNumPitstops),
                offset_of!(VehicleScoring, mVehicleClass),
                offset_of!(VehicleScoring, mTimeBehindNext),
                offset_of!(VehicleScoring, mPos),
                offset_of!(VehicleScoring, mHeadlights),
                offset_of!(VehicleScoring, mQualification),
                offset_of!(VehicleScoring, mPitGroup),
                offset_of!(VehicleScoring, mFlag),
                offset_of!(VehicleScoring, mPitLapDist),
                offset_of!(VehicleScoring, mSteamID),
                offset_of!(VehicleScoring, mVehFilename),
                offset_of!(VehicleScoring, mExpansion),
            ],
            [100, 104, 192, 200, 232, 264, 456, 460, 480, 504, 524, 536, 544, 580]
        );
        assert_eq!(
            [
                offset_of!(VehicleTelemetry, mLapNumber),
                offset_of!(VehicleTelemetry, mVehicleName),
                offset_of!(VehicleTelemetry, mPos),
                offset_of!(VehicleTelemetry, mGear),
                offset_of!(VehicleTelemetry, mFuel),
                offset_of!(VehicleTelemetry, mScheduledStops),
                offset_of!(VehicleTelemetry, mLastImpactET),
                offset_of!(VehicleTelemetry, mCurrentSector),
                offset_of!(VehicleTelemetry, mFuelCapacity),
                offset_of!(VehicleTelemetry, mFrontTireCompoundName),
                offset_of!(VehicleTelemetry, mVisualSteeringWheelRange),
                offset_of!(VehicleTelemetry, mPhysicalSteeringWheelRange),
                offset_of!(VehicleTelemetry, mDeltaBest),
                offset_of!(VehicleTelemetry, mElectricBoostMotorState),
                offset_of!(VehicleTelemetry, mRegen),
                offset_of!(VehicleTelemetry, mVehicleModel),
                offset_of!(VehicleTelemetry, mWheels),
            ],
            [20, 32, 160, 352, 524, 540, 552, 600, 608, 620, 660, 692, 696, 744, 768, 796, 848]
        );
        assert_eq!(
            [
                offset_of!(Wheel, mTerrainName),
                offset_of!(Wheel, mSurfaceType),
                offset_of!(Wheel, mVerticalTireDeflection),
                offset_of!(Wheel, mOptimalTemp),
                offset_of!(Wheel, mCompoundType)
            ],
            [160, 176, 180, 236, 241]
        );
        assert_eq!([offset_of!(Generic, gameVersion), offset_of!(Generic, appInfo)], [64, 72]);
    }

    #[test]
    fn reads_text_fields() {
        let mut b = [0u8; 16];
        b[..5].copy_from_slice(b"Le Ma");
        assert_eq!(text(&b), "Le Ma");
        // Not UTF-8: read as Latin-1.
        assert_eq!(text(&[b'P', 0xe9, b'r', 0]), "Pér");
    }
}
