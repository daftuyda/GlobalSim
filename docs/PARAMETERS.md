# Complete parameter reference

Parameter values used by the engine. These tables do not imply every
parameter is consumed in every runtime mode. Follow `Parameters::from_value`
and the linked modules to see usage. Float values retain their JSON precision.

## Captured Global parameters

| Path | Value |
|---|---|
| `paramDefineVersion` | `"20180828"` |
| `Global.FinishTimeCoef` | `1.1799999475479126` |
| `Global.SingleModeAddSpeed` | `400` |
| `Global.SingleModeAddStamina` | `400` |
| `Global.SingleModeAddPow` | `400` |
| `Global.SingleModeAddGuts` | `400` |
| `Global.SingleModeAddWiz` | `400` |
| `raceBaseSpeedBaseSpeed` | `20` |
| `raceBaseSpeedBaseDistance` | `2000` |
| `raceBaseSpeedDistanceRate` | `1000` |
| `accelPowCoef` | `0.0006000000284984708` |
| `accelPowCoefUpSlope` | `0.00039999998989515007` |
| `AccelPowCoefSqrt` | `500` |
| `declBase` | `1` |
| `declRateStart` | `1.2000000476837158` |
| `declRateMiddle` | `0.800000011920929` |
| `declRateEnd` | `1` |
| `declRateHpZero` | `1.2000000476837158` |
| `DeclRatePositionKeepPaseDown` | `0.5` |
| `forceInMoveLaneThreshold` | `0.11999999731779099` |
| `forceInMoveTargetSpeedAddRunningOut` | `0.019999999552965164` |
| `forceInMoveTargetSpeedAddProceding` | `0.009999999776482582` |
| `forceInMoveTargetSpeedAddInsertion` | `0.009999999776482582` |
| `forceInMoveTargetSpeedAddChasing` | `0.029999999329447746` |
| `forceInMoveTargetSpeedAddRandomRange` | `0.10000000149011612` |
| `addSpeedParamCoef` | `0.0020000000949949026` |
| `baseTargetSpeedCoef` | `1.0499999523162842` |
| `LastSpurtTargetSpeedCoefSqrt` | `500` |
| `LastSpurtDistanceGoalBuffer` | `60` |
| `lastSpurtSpeedDelta` | `0.10000000149011612` |
| `lastSpurtDistanceMaxFromGoal` | `100` |
| `lastSpurtStep2SpeedDelta` | `0.10000000149011612` |
| `lastSpurtStep2PerVal1` | `15` |
| `lastSpurtStep2PerVal2` | `0.05000000074505806` |
| `LastSpurtTargetSpeedGutsCoef1` | `450` |
| `LastSpurtTargetSpeedGutsPow` | `0.597000002861023` |
| `LastSpurtTargetSpeedGutsCoef2` | `9.999999747378752e-05` |
| `StartSpeed` | `3` |
| `StartAccelAdd` | `24` |
| `Speed.MinSpeedRate` | `0.8500000238418579` |
| `Speed.MinSpeedGutsCoefSqrt` | `200` |
| `Speed.MinSpeedGutsCoef` | `0.0010000000474974513` |
| `Speed.TargetSpeedMin` | `13` |
| `Speed.ExpectLastSpurtSpeed` | `21` |
| `Speed.PhaseAccelCoefArray[0].Start` | `1` |
| `Speed.PhaseAccelCoefArray[0].Middle` | `1` |
| `Speed.PhaseAccelCoefArray[0].End` | `0.9959999918937683` |
| `Speed.PhaseAccelCoefArray[0].Last` | `0.9959999918937683` |
| `Speed.PhaseAccelCoefArray[1].Start` | `0.9850000143051147` |
| `Speed.PhaseAccelCoefArray[1].Middle` | `1` |
| `Speed.PhaseAccelCoefArray[1].End` | `0.9959999918937683` |
| `Speed.PhaseAccelCoefArray[1].Last` | `0.9959999918937683` |
| `Speed.PhaseAccelCoefArray[2].Start` | `0.9750000238418579` |
| `Speed.PhaseAccelCoefArray[2].Middle` | `1` |
| `Speed.PhaseAccelCoefArray[2].End` | `1` |
| `Speed.PhaseAccelCoefArray[2].Last` | `1` |
| `Speed.PhaseAccelCoefArray[3].Start` | `0.9449999928474426` |
| `Speed.PhaseAccelCoefArray[3].Middle` | `1` |
| `Speed.PhaseAccelCoefArray[3].End` | `0.996999979019165` |
| `Speed.PhaseAccelCoefArray[3].Last` | `0.996999979019165` |
| `Speed.PhaseAccelCoefExArray[0].Start` | `1.1699999570846558` |
| `Speed.PhaseAccelCoefExArray[0].Middle` | `0.9399999976158142` |
| `Speed.PhaseAccelCoefExArray[0].End` | `0.9559999704360962` |
| `Speed.PhaseAccelCoefExArray[0].Last` | `0.9559999704360962` |
| `CourseSetAdjust.CoefAndThresholdArray[0].Coef` | `0.05000000074505806` |
| `CourseSetAdjust.CoefAndThresholdArray[0].StatusThreshold` | `300` |
| `CourseSetAdjust.CoefAndThresholdArray[1].Coef` | `0.10000000149011612` |
| `CourseSetAdjust.CoefAndThresholdArray[1].StatusThreshold` | `600` |
| `CourseSetAdjust.CoefAndThresholdArray[2].Coef` | `0.15000000596046448` |
| `CourseSetAdjust.CoefAndThresholdArray[2].StatusThreshold` | `900` |
| `CourseSetAdjust.CoefAndThresholdArray[3].Coef` | `0.20000000298023224` |
| `CourseSetAdjust.CoefAndThresholdArray[3].StatusThreshold` | `2000` |
| `Slope.SlopePerThreshold` | `0` |
| `Slope.UpSlopeAddSpeedVal1` | `200` |
| `Slope.DownSlopeAddSpeedVal1` | `0.30000001192092896` |
| `Slope.DownSlopeAddSpeedVal2` | `10` |
| `Slope.DownSlopeAddSpeedStartWizRate` | `0.03999999910593033` |
| `Slope.DownSlopeAddSpeedEndPer` | `20` |
| `lastSpurtBaseTargetSpeedAddCoef` | `0.009999999776482582` |
| `_groundModifierParamTurf[0].addSpeed` | `0` |
| `_groundModifierParamTurf[0].addPower` | `0` |
| `_groundModifierParamTurf[0].multiHpSub` | `1` |
| `_groundModifierParamTurf[1].addSpeed` | `0` |
| `_groundModifierParamTurf[1].addPower` | `-50` |
| `_groundModifierParamTurf[1].multiHpSub` | `1` |
| `_groundModifierParamTurf[2].addSpeed` | `0` |
| `_groundModifierParamTurf[2].addPower` | `-50` |
| `_groundModifierParamTurf[2].multiHpSub` | `1.0199999809265137` |
| `_groundModifierParamTurf[3].addSpeed` | `-50` |
| `_groundModifierParamTurf[3].addPower` | `-50` |
| `_groundModifierParamTurf[3].multiHpSub` | `1.0199999809265137` |
| `_groundModifierParamDirt[0].addSpeed` | `0` |
| `_groundModifierParamDirt[0].addPower` | `-100` |
| `_groundModifierParamDirt[0].multiHpSub` | `1` |
| `_groundModifierParamDirt[1].addSpeed` | `0` |
| `_groundModifierParamDirt[1].addPower` | `-50` |
| `_groundModifierParamDirt[1].multiHpSub` | `1` |
| `_groundModifierParamDirt[2].addSpeed` | `0` |
| `_groundModifierParamDirt[2].addPower` | `-100` |
| `_groundModifierParamDirt[2].multiHpSub` | `1.0099999904632568` |
| `_groundModifierParamDirt[3].addSpeed` | `-50` |
| `_groundModifierParamDirt[3].addPower` | `-100` |
| `_groundModifierParamDirt[3].multiHpSub` | `1.0199999809265137` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[0].Start` | `100` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[0].Middle` | `98` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[0].End` | `96.19999694824219` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[0].Last` | `96.19999694824219` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[1].Start` | `97.80000305175781` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[1].Middle` | `99.0999984741211` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[1].End` | `97.5` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[1].Last` | `97.5` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[2].Start` | `93.80000305175781` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[2].Middle` | `99.80000305175781` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[2].End` | `99.4000015258789` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[2].Last` | `99.4000015258789` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[3].Start` | `93.0999984741211` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[3].Middle` | `100` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[3].End` | `100` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerArray[3].Last` | `100` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerExArray[0].Start` | `106.30000305175781` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerExArray[0].Middle` | `96.19999694824219` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerExArray[0].End` | `95` |
| `BasetTargetSpeed.PhaseBaseTargetSpeedPerExArray[0].Last` | `95` |
| `BasetTargetSpeed.BaseTargetSpeedRandomMinusVal1` | `-0.6499999761581421` |
| `BasetTargetSpeed.BaseTargetSpeedRandomMinusVal2` | `5500` |
| `BasetTargetSpeed.BaseTargetSpeedRandomPlusVal1` | `5500` |
| `BasetTargetSpeed.PhaseEndBaseTargetSpeedCoef` | `500` |
| `laneMoveSpeedBase` | `0.019999999552965164` |
| `laneMoveSpeedAdd` | `0.30000001192092896` |
| `laneMoveSpeedPowCoef` | `0.0010000000474974513` |
| `laneMoveSpeedOutCoef` | `0.05000000074505806` |
| `CongestionLaneGapAbs` | `0.30000001192092896` |
| `CongestionHorseCntThreshold` | `3` |
| `laneMoveAccelMoveSpeedBaseCoef` | `1.5` |
| `lastMoveOutLaneNormalizeVal` | `0.10000000149011612` |
| `lastMoveOutLaneCoef` | `0.5` |
| `lastMoveOutLaneAddRandom` | `0.10000000149011612` |
| `LastMoveOutStartFinalCornerRate` | `0` |
| `Block.FrontBlockDistanceGap` | `2` |
| `Block.FrontBlockLaneGap` | `0.0416666716337204` |
| `Block.SideBlockDistanceGap` | `1.0499999523162842` |
| `Block.SideBlockLaneGap` | `0.1111110970377922` |
| `Block.FrontBlockMaxSpeedRateMin` | `0.9879999756813049` |
| `Block.FrontBlockMaxSpeedRateMax` | `1` |
| `Near.DistanceThreshold` | `3` |
| `Near.LaneDistanceThreshold` | `0.16665999591350555` |
| `Hp.HpInitialVal1` | `0.800000011920929` |
| `Hp.HpDecBase` | `20` |
| `Hp.HpMaxCoefOoNige` | `0.8600000143051147` |
| `Hp.HpMaxCoefNige` | `0.949999988079071` |
| `Hp.HpMaxCoefSenko` | `0.8899999856948853` |
| `Hp.HpMaxCoefSashi` | `1` |
| `Hp.HpMaxCoefOikomi` | `0.9950000047683716` |
| `Hp.SpeedGapParam1` | `12` |
| `Hp.SpeedGapParam1Pow` | `144` |
| `Hp.HpDecRateBaseNormal` | `1` |
| `Hp.HpDecRateBaseTemptation` | `1.600000023841858` |
| `Hp.HpDecRateBasePositionKeepPaseDown` | `0.6000000238418579` |
| `Hp.HpDecRateBaseCompeteTopNige` | `1.399999976158142` |
| `Hp.HpDecRateBaseTemptationAndCompeteTopNige` | `3.5999999046325684` |
| `Hp.HpDecRateBaseCompeteTopOonige` | `3.5` |
| `Hp.HpDecRateBaseTemptationAndCompeteTopOonige` | `7.699999809265137` |
| `Hp.HpDecRateMultiplyDownSlopeAccelMode` | `0.4000000059604645` |
| `Hp.HpGutsCoef` | `200` |
| `Hp.HpGutsCoefSqrt` | `600` |
| `PositionKeep.PositionKeepCheckIntervalSec` | `2` |
| `PositionKeep.PositionKeepCoolDownSec` | `1` |
| `PositionKeep.PositionKeepStartSection` | `1` |
| `PositionKeep.PositionKeepEndSection` | `10` |
| `PositionKeep.PositionKeepContinueSection` | `1` |
| `PositionKeep.PositionKeepContinueSectionExArray[0]` | `3` |
| `PositionKeep.PositionKeepSpeedUpStartPerVal1` | `20` |
| `PositionKeep.PositionKeepSpeedUpBaseTargetSpeedMultiply` | `1.0399999618530273` |
| `PositionKeep.PositionKeepSpeedUpEndDistanceDiff` | `4.5` |
| `PositionKeep.PositionKeepSpeedUpEndDistanceDiffOne` | `12.5` |
| `PositionKeep.PositionKeepOvertakeStartPerVal1` | `20` |
| `PositionKeep.PositionKeepOvertakeEndDistanceDiff` | `10` |
| `PositionKeep.PositionKeepOvertakeBaseTargetSpeedMultiply` | `1.0499999523162842` |
| `PositionKeep.PositionKeepSpeedUpStartPerVal1Oonige` | `20` |
| `PositionKeep.PositionKeepSpeedUpBaseTargetSpeedMultiplyOonige` | `1.0399999618530273` |
| `PositionKeep.PositionKeepSpeedUpEndDistanceDiffOonige` | `17.5` |
| `PositionKeep.PositionKeepOvertakeStartPerVal1Oonige` | `20` |
| `PositionKeep.PositionKeepOvertakeEndDistanceDiffOonige` | `27.5` |
| `PositionKeep.PositionKeepOvertakeBaseTargetSpeedMultiplyOonige` | `1.0499999523162842` |
| `PositionKeep.PositionKeepPaseUpDownDistanceDiffMaxCoef` | `0.5` |
| `PositionKeep.PositionKeepPaseUpDownDistanceDiffMinSenko` | `3` |
| `PositionKeep.PositionKeepPaseUpDownDistanceDiffMaxSenko` | `5` |
| `PositionKeep.PositionKeepPaseUpDownDistanceDiffMinSashi` | `6.5` |
| `PositionKeep.PositionKeepPaseUpDownDistanceDiffMaxSashi` | `7` |
| `PositionKeep.PositionKeepPaseUpDownDistanceDiffMinOikomi` | `7.5` |
| `PositionKeep.PositionKeepPaseUpDownDistanceDiffMaxOikomi` | `8` |
| `PositionKeep.PositionKeepPaseBaseDistance` | `1000` |
| `PositionKeep.PositionKeepPaseBaseDistanceCoef` | `0.0007999999797903001` |
| `PositionKeep.PositionKeepPaseDownBaseTargetSpeedMultiply` | `0.9150000214576721` |
| `PositionKeep.PositionKeepPaseDownBaseTargetSpeedMultiplyPhaseMiddle` | `0.9449999928474426` |
| `PositionKeep.PositionKeepPaseDownTargetLane` | `0.18000000715255737` |
| `PositionKeep.PositionKeepPaseUpStartPerVal1` | `15` |
| `PositionKeep.PositionKeepPaseUpBaseTargetSpeedMultiply` | `1.0399999618530273` |
| `PositionKeep.PositionKeepPaseUpExBaseTargetSpeedMultiply` | `2` |
| `PositionKeep.PaseMakerChangeForwardRunningStyleRange` | `10` |
| `PositionKeep.PaseMakerChangeMostForwardStyleRange` | `10` |
| `PositionKeep.PaseMakerChangeTopHorseNotMostForwardStyleCount` | `2` |
| `startDelayMax` | `0.10000000149011612` |
| `ConservePower.ReleaseConservePowerAccelCoef` | `0.0010000000474974513` |
| `ConservePower.ReleaseConservePowerDeclCoef` | `130` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[0].RunningStyle` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[0].DistanceType` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[0].Coef` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[1].RunningStyle` | `2` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[1].DistanceType` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[1].Coef` | `0.699999988079071` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[2].RunningStyle` | `3` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[2].DistanceType` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[2].Coef` | `0.75` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[3].RunningStyle` | `4` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[3].DistanceType` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[3].Coef` | `0.699999988079071` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[4].RunningStyle` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[4].DistanceType` | `2` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[4].Coef` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[5].RunningStyle` | `2` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[5].DistanceType` | `2` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[5].Coef` | `0.800000011920929` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[6].RunningStyle` | `3` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[6].DistanceType` | `2` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[6].Coef` | `0.699999988079071` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[7].RunningStyle` | `4` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[7].DistanceType` | `2` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[7].Coef` | `0.75` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[8].RunningStyle` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[8].DistanceType` | `3` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[8].Coef` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[9].RunningStyle` | `2` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[9].DistanceType` | `3` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[9].Coef` | `0.8999999761581421` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[10].RunningStyle` | `3` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[10].DistanceType` | `3` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[10].Coef` | `0.875` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[11].RunningStyle` | `4` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[11].DistanceType` | `3` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[11].Coef` | `0.8600000143051147` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[12].RunningStyle` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[12].DistanceType` | `4` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[12].Coef` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[13].RunningStyle` | `2` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[13].DistanceType` | `4` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[13].Coef` | `0.8999999761581421` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[14].RunningStyle` | `3` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[14].DistanceType` | `4` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[14].Coef` | `1` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[15].RunningStyle` | `4` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[15].DistanceType` | `4` |
| `ConservePower.ReleaseConservePowerAccelRunningStyleAndDistanceTypeCoefArray[15].Coef` | `0.8999999761581421` |
| `ConservePower.ConservePowerDecreaseAccelCoefArray[0].Mode` | `3` |
| `ConservePower.ConservePowerDecreaseAccelCoefArray[0].Coef` | `0.9800000190734863` |
| `ConservePower.ConservePowerDecreaseAccelCoefArray[1].Mode` | `4` |
| `ConservePower.ConservePowerDecreaseAccelCoefArray[1].Coef` | `0.800000011920929` |
| `ConservePower.ReleaseConservePowerActivityTimeCoef` | `1450` |
| `ConservePower.ReleaseConservePowerActivityTimeDistanceTypeCoefArray[0].DistanceType` | `1` |
| `ConservePower.ReleaseConservePowerActivityTimeDistanceTypeCoefArray[0].Coef` | `0.44999998807907104` |
| `ConservePower.ReleaseConservePowerActivityTimeDistanceTypeCoefArray[1].DistanceType` | `2` |
| `ConservePower.ReleaseConservePowerActivityTimeDistanceTypeCoefArray[1].Coef` | `1` |
| `ConservePower.ReleaseConservePowerActivityTimeDistanceTypeCoefArray[2].DistanceType` | `3` |
| `ConservePower.ReleaseConservePowerActivityTimeDistanceTypeCoefArray[2].Coef` | `0.875` |
| `ConservePower.ReleaseConservePowerActivityTimeDistanceTypeCoefArray[3].DistanceType` | `4` |
| `ConservePower.ReleaseConservePowerActivityTimeDistanceTypeCoefArray[3].Coef` | `0.800000011920929` |
| `ConservePower.ConservePowerDefaultCoolDownTime` | `1.5` |
| `ConservePower.ConservePowerIncreaseCoefArray[0].Mode` | `1` |
| `ConservePower.ConservePowerIncreaseCoefArray[0].Coef` | `6.699999809265137` |
| `ConservePower.ConservePowerIncreaseCoefArray[1].Mode` | `2` |
| `ConservePower.ConservePowerIncreaseCoefArray[1].Coef` | `4.199999809265137` |
| `ConservePower.ConservePowerRunningStyleCoefArray[0].RunningStyle` | `1` |
| `ConservePower.ConservePowerRunningStyleCoefArray[0].Coef` | `1` |
| `ConservePower.ConservePowerRunningStyleCoefArray[1].RunningStyle` | `2` |
| `ConservePower.ConservePowerRunningStyleCoefArray[1].Coef` | `0.75` |
| `ConservePower.ConservePowerRunningStyleCoefArray[2].RunningStyle` | `3` |
| `ConservePower.ConservePowerRunningStyleCoefArray[2].Coef` | `1` |
| `ConservePower.ConservePowerRunningStyleCoefArray[3].RunningStyle` | `4` |
| `ConservePower.ConservePowerRunningStyleCoefArray[3].Coef` | `0.699999988079071` |
| `ConservePower.ConservePowerDecreaseCoefArray[0].Mode` | `3` |
| `ConservePower.ConservePowerDecreaseCoefArray[0].Coef` | `0.949999988079071` |
| `ConservePower.ConservePowerDecreaseCoefArray[1].Mode` | `4` |
| `ConservePower.ConservePowerDecreaseCoefArray[1].Coef` | `0.800000011920929` |
| `ConservePower.ConservePowerReleasableActivityTimeThreshold` | `0.20000000298023224` |
| `Temptation.TemptationLotSectionMin` | `2` |
| `Temptation.TemptationLotSectionMax` | `9` |
| `Temptation.TemptationStartPerVal1` | `13` |
| `Temptation.TemptationForceEndTime` | `12` |
| `Temptation.TemptationEndCheckTime` | `3` |
| `Temptation.TemptationEndPer` | `55` |
| `Temptation.OikomiToSashiPer` | `10` |
| `Temptation.OikomiToSenkoPer` | `20` |
| `Temptation.OikomiToNigePer` | `70` |
| `Temptation.SashiToSenkoPer` | `25` |
| `Temptation.SashiToNigePer` | `75` |
| `Temptation.SenkoToNigePer` | `100` |
| `CompeteTop.CheckStartDistance` | `150` |
| `CompeteTop.CheckEndSection` | `6` |
| `CompeteTop.EndSection` | `9` |
| `CompeteTop.NigeCount` | `1` |
| `CompeteTop.OonigeCount` | `1` |
| `CompeteTop.DistanceGap1` | `3.75` |
| `CompeteTop.LaneGap1` | `0.16500000655651093` |
| `CompeteTop.DistanceGap2` | `5` |
| `CompeteTop.LaneGap2` | `0.41600000858306885` |
| `CompeteTop.TimeCoef1` | `700` |
| `CompeteTop.TimeCoef2` | `0.5` |
| `CompeteTop.TimeCoef3` | `0.012000000104308128` |
| `CompeteTop.AddParam1Coef1` | `500` |
| `CompeteTop.AddParam1Coef2` | `0.6000000238418579` |
| `CompeteTop.AddParam1Coef3` | `9.999999747378752e-05` |
| `CompeteFight.DistanceGap` | `3` |
| `CompeteFight.LaneGap` | `0.25` |
| `CompeteFight.TargetContinueTime` | `2` |
| `CompeteFight.SpeedGap` | `0.6000000238418579` |
| `CompeteFight.TargetOrderPer` | `50` |
| `CompeteFight.TargetContinueDistance` | `5` |
| `CompeteFight.HpPer` | `5` |
| `CompeteFight.HpPer2` | `15` |
| `CompeteFight.AddParam1Coef1` | `200` |
| `CompeteFight.AddParam1Coef2` | `0.7080000042915344` |
| `CompeteFight.AddParam1Coef3` | `9.999999747378752e-05` |
| `CompeteFight.AddParam2Coef1` | `160` |
| `CompeteFight.AddParam2Coef2` | `0.5899999737739563` |
| `CompeteFight.AddParam2Coef3` | `9.999999747378752e-05` |
| `Surrounded.DistanceGapAbs1` | `1.5` |
| `Surrounded.LaneGap1` | `0.16665999591350555` |
| `Surrounded.DistanceGap2` | `3` |
| `Surrounded.LaneGapAbs2` | `0.08332999795675278` |
| `visibleDistance` | `20` |
| `overTakeDistPerSpeed` | `15` |
| `overTakeCoolDownTime` | `1.5` |
| `overTakeCountCoolDownTime` | `6` |
| `overTakeInLaneCoefEnd` | `1` |
| `overTakeOutLaneCoefEnd` | `1.149999976158142` |
| `Skill.AbilityTimeDivideDistance` | `1000` |
| `Skill.CoolDownTimeDivideDistance` | `1000` |
| `Skill.RandomLotOtherActivateLotInterval` | `1` |
| `Skill.LotActivatePerVal1` | `100` |
| `Skill.LotActivatePerVal2` | `9000` |
| `Skill.InfrontHorseNearDistance` | `2.5` |
| `Skill.BehindHorseNearDistance` | `2.5` |
| `Skill.InfrontHorseNearLaneDistance` | `0.05559999868273735` |
| `Skill.BehindHorseNearLaneDistance` | `0.05559999868273735` |
| `Skill.ActivatePerMin` | `20` |
| `Skill.TeamTotalStatusCoefAscArray[0].StatusThreshold` | `1200` |
| `Skill.TeamTotalStatusCoefAscArray[0].Multiply` | `0.800000011920929` |
| `Skill.TeamTotalStatusCoefAscArray[1].StatusThreshold` | `1800` |
| `Skill.TeamTotalStatusCoefAscArray[1].Multiply` | `0.8999999761581421` |
| `Skill.TeamTotalStatusCoefAscArray[2].StatusThreshold` | `2600` |
| `Skill.TeamTotalStatusCoefAscArray[2].Multiply` | `1` |
| `Skill.TeamTotalStatusCoefAscArray[3].StatusThreshold` | `3600` |
| `Skill.TeamTotalStatusCoefAscArray[3].Multiply` | `1.100000023841858` |
| `Skill.TeamTotalStatusCoefAscArray[4].StatusThreshold` | `99999` |
| `Skill.TeamTotalStatusCoefAscArray[4].Multiply` | `1.2000000476837158` |
| `Skill.AbilityTimeDiffDistanceTopCoefPerDistance` | `62.5` |
| `Skill.AbilityTimeDiffDistanceTopCoefMin` | `0.800000011920929` |
| `Skill.AbilityTimeDiffDistanceTopCoefMax` | `1.600000023841858` |
| `Skill.AbiiltyValueUsageRandom1Array[0].AbilityValueCoef` | `0` |
| `Skill.AbiiltyValueUsageRandom1Array[0].Per` | `60` |
| `Skill.AbiiltyValueUsageRandom1Array[1].AbilityValueCoef` | `0.019999999552965164` |
| `Skill.AbiiltyValueUsageRandom1Array[1].Per` | `30` |
| `Skill.AbiiltyValueUsageRandom1Array[2].AbilityValueCoef` | `0.03999999910593033` |
| `Skill.AbiiltyValueUsageRandom1Array[2].Per` | `10` |
| `Skill.AbiiltyValueUsageRandom2Array[0].AbilityValueCoef` | `0` |
| `Skill.AbiiltyValueUsageRandom2Array[0].Per` | `60` |
| `Skill.AbiiltyValueUsageRandom2Array[1].AbilityValueCoef` | `0.019999999552965164` |
| `Skill.AbiiltyValueUsageRandom2Array[1].Per` | `30` |
| `Skill.AbiiltyValueUsageRandom2Array[2].AbilityValueCoef` | `0.03999999910593033` |
| `Skill.AbiiltyValueUsageRandom2Array[2].Per` | `10` |
| `Skill.BehindNearParamArray[0].Distance` | `5` |
| `Skill.BehindNearParamArray[0].LaneDistance` | `0.15000000596046448` |
| `Skill.LaneMoveAddParam1` | `0.00019999999494757503` |
| `Skill.LaneMoveAddParam2` | `0.5` |
| `Skill.TimeRemainHpCoefAscArray[0].HpThreshold` | `2000` |
| `Skill.TimeRemainHpCoefAscArray[0].Multiply` | `1` |
| `Skill.TimeRemainHpCoefAscArray[1].HpThreshold` | `2400` |
| `Skill.TimeRemainHpCoefAscArray[1].Multiply` | `1.5` |
| `Skill.TimeRemainHpCoefAscArray[2].HpThreshold` | `2600` |
| `Skill.TimeRemainHpCoefAscArray[2].Multiply` | `2` |
| `Skill.TimeRemainHpCoefAscArray[3].HpThreshold` | `2800` |
| `Skill.TimeRemainHpCoefAscArray[3].Multiply` | `2.200000047683716` |
| `Skill.TimeRemainHpCoefAscArray[4].HpThreshold` | `3000` |
| `Skill.TimeRemainHpCoefAscArray[4].Multiply` | `2.5` |
| `Skill.TimeRemainHpCoefAscArray[5].HpThreshold` | `3200` |
| `Skill.TimeRemainHpCoefAscArray[5].Multiply` | `3` |
| `Skill.TimeRemainHpCoefAscArray[6].HpThreshold` | `3500` |
| `Skill.TimeRemainHpCoefAscArray[6].Multiply` | `3.5` |
| `Skill.TimeRemainHpCoefAscArray[7].HpThreshold` | `99999` |
| `Skill.TimeRemainHpCoefAscArray[7].Multiply` | `4` |
| `Skill.TimeRemainHpCoefAscArray2[0].HpThreshold` | `1500` |
| `Skill.TimeRemainHpCoefAscArray2[0].Multiply` | `1` |
| `Skill.TimeRemainHpCoefAscArray2[1].HpThreshold` | `1800` |
| `Skill.TimeRemainHpCoefAscArray2[1].Multiply` | `1.5` |
| `Skill.TimeRemainHpCoefAscArray2[2].HpThreshold` | `2000` |
| `Skill.TimeRemainHpCoefAscArray2[2].Multiply` | `2` |
| `Skill.TimeRemainHpCoefAscArray2[3].HpThreshold` | `2100` |
| `Skill.TimeRemainHpCoefAscArray2[3].Multiply` | `2.5` |
| `Skill.TimeRemainHpCoefAscArray2[4].HpThreshold` | `99999` |
| `Skill.TimeRemainHpCoefAscArray2[4].Multiply` | `3` |
| `Skill.RaceWinCountCoefAscArray[0].WinCountThreshold` | `6` |
| `Skill.RaceWinCountCoefAscArray[0].Multiply` | `0.800000011920929` |
| `Skill.RaceWinCountCoefAscArray[1].WinCountThreshold` | `14` |
| `Skill.RaceWinCountCoefAscArray[1].Multiply` | `0.8999999761581421` |
| `Skill.RaceWinCountCoefAscArray[2].WinCountThreshold` | `18` |
| `Skill.RaceWinCountCoefAscArray[2].Multiply` | `1` |
| `Skill.RaceWinCountCoefAscArray[3].WinCountThreshold` | `25` |
| `Skill.RaceWinCountCoefAscArray[3].Multiply` | `1.100000023841858` |
| `Skill.RaceWinCountCoefAscArray[4].WinCountThreshold` | `99999` |
| `Skill.RaceWinCountCoefAscArray[4].Multiply` | `1.2000000476837158` |
| `Skill.OrderUpCornerPhaseEndAfterCoefAscArray[0].OrderChangeCountThreshold` | `2` |
| `Skill.OrderUpCornerPhaseEndAfterCoefAscArray[0].Multiply` | `1` |
| `Skill.OrderUpCornerPhaseEndAfterCoefAscArray[1].OrderChangeCountThreshold` | `3` |
| `Skill.OrderUpCornerPhaseEndAfterCoefAscArray[1].Multiply` | `1.100000023841858` |
| `Skill.OrderUpCornerPhaseEndAfterCoefAscArray[2].OrderChangeCountThreshold` | `4` |
| `Skill.OrderUpCornerPhaseEndAfterCoefAscArray[2].Multiply` | `1.2000000476837158` |
| `Skill.OrderUpCornerPhaseEndAfterCoefAscArray[3].OrderChangeCountThreshold` | `99` |
| `Skill.OrderUpCornerPhaseEndAfterCoefAscArray[3].Multiply` | `1.25` |
| `Skill.BaseWizCoefAscArray[0].BaseStatusThreshold` | `500` |
| `Skill.BaseWizCoefAscArray[0].Multiply` | `0.5` |
| `Skill.BaseWizCoefAscArray[1].BaseStatusThreshold` | `800` |
| `Skill.BaseWizCoefAscArray[1].Multiply` | `1` |
| `Skill.BaseWizCoefAscArray[2].BaseStatusThreshold` | `1000` |
| `Skill.BaseWizCoefAscArray[2].Multiply` | `1.100000023841858` |
| `Skill.BaseWizCoefAscArray[3].BaseStatusThreshold` | `1200` |
| `Skill.BaseWizCoefAscArray[3].Multiply` | `1.2000000476837158` |
| `Skill.BaseWizCoefAscArray[4].BaseStatusThreshold` | `1300` |
| `Skill.BaseWizCoefAscArray[4].Multiply` | `1.2999999523162842` |
| `Skill.BaseWizCoefAscArray[5].BaseStatusThreshold` | `9999` |
| `Skill.BaseWizCoefAscArray[5].Multiply` | `1.399999976158142` |
| `Skill.MaximumRawStatusCoefAscArray[0].BaseStatusThreshold` | `600` |
| `Skill.MaximumRawStatusCoefAscArray[0].Multiply` | `0.800000011920929` |
| `Skill.MaximumRawStatusCoefAscArray[1].BaseStatusThreshold` | `800` |
| `Skill.MaximumRawStatusCoefAscArray[1].Multiply` | `0.8999999761581421` |
| `Skill.MaximumRawStatusCoefAscArray[2].BaseStatusThreshold` | `1000` |
| `Skill.MaximumRawStatusCoefAscArray[2].Multiply` | `1` |
| `Skill.MaximumRawStatusCoefAscArray[3].BaseStatusThreshold` | `1100` |
| `Skill.MaximumRawStatusCoefAscArray[3].Multiply` | `1.100000023841858` |
| `Skill.MaximumRawStatusCoefAscArray[4].BaseStatusThreshold` | `9999` |
| `Skill.MaximumRawStatusCoefAscArray[4].Multiply` | `1.2000000476837158` |
| `Skill.OrderUpAddAbilityTime` | `1` |
| `Skill.OrderUpAddAbilityTimeMaxCount` | `3` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[0]` | `101` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[1]` | `102` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[2]` | `103` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[3]` | `104` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[4]` | `201` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[5]` | `202` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[6]` | `203` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[7]` | `204` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[8]` | `301` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[9]` | `302` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[10]` | `303` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[11]` | `501` |
| `Skill.SkillActivateTagGroupArray[0].TagArray[12]` | `502` |
| `Skill.SkillActivateTagGroupArray[1].TagArray[0]` | `403` |
| `Skill.AbilityValueUsageTagGroup1Array[0]` | `601` |
| `Skill.AbilityValueUsageTagGroup1Array[1]` | `602` |
| `Skill.AbilityValueUsageTagGroup1Array[2]` | `603` |
| `Skill.AbilityValueUsageTagGroup1Array[3]` | `604` |
| `Skill.AbilityValueUsageTagGroup1Array[4]` | `605` |
| `Skill.AbilityValueUsageTagGroup1Array[5]` | `606` |
| `Skill.AbilityValueUsageTagGroup1Array[6]` | `607` |
| `Skill.AbilityValueUsageTagGroup1Array[7]` | `608` |
| `Skill.AbilityValueUsageTagGroup1Array[8]` | `609` |
| `Skill.AbilityValueUsageTagGroup1Array[9]` | `610` |
| `Skill.AbilityValueUsageTagGroup1Array[10]` | `611` |
| `Skill.AbilityValueUsageTagGroup1Array[11]` | `612` |
| `Skill.AbilityValueUsageTagGroup1Array[12]` | `613` |
| `Skill.AbilityValueUsageTagGroup1Array[13]` | `614` |
| `Skill.AbilityValueUsageTagGroup1Array[14]` | `615` |
| `Skill.ActivateSpecificTagGroupSkillCountCoefAscArray[0].ActivateSpecificTagGroupSkillCountThreshold` | `3` |
| `Skill.ActivateSpecificTagGroupSkillCountCoefAscArray[0].Multiply` | `0` |
| `Skill.ActivateSpecificTagGroupSkillCountCoefAscArray[1].ActivateSpecificTagGroupSkillCountThreshold` | `5` |
| `Skill.ActivateSpecificTagGroupSkillCountCoefAscArray[1].Multiply` | `1` |
| `Skill.ActivateSpecificTagGroupSkillCountCoefAscArray[2].ActivateSpecificTagGroupSkillCountThreshold` | `6` |
| `Skill.ActivateSpecificTagGroupSkillCountCoefAscArray[2].Multiply` | `2` |
| `Skill.ActivateSpecificTagGroupSkillCountCoefAscArray[3].ActivateSpecificTagGroupSkillCountThreshold` | `99` |
| `Skill.ActivateSpecificTagGroupSkillCountCoefAscArray[3].Multiply` | `3` |
| `Skill.FanCountCoefAscArray[0].FanCountThreshold` | `20000` |
| `Skill.FanCountCoefAscArray[0].Multiply` | `0.800000011920929` |
| `Skill.FanCountCoefAscArray[1].FanCountThreshold` | `50000` |
| `Skill.FanCountCoefAscArray[1].Multiply` | `0.8999999761581421` |
| `Skill.FanCountCoefAscArray[2].FanCountThreshold` | `100000` |
| `Skill.FanCountCoefAscArray[2].Multiply` | `1` |
| `Skill.FanCountCoefAscArray[3].FanCountThreshold` | `160000` |
| `Skill.FanCountCoefAscArray[3].Multiply` | `1.100000023841858` |
| `Skill.FanCountCoefAscArray[4].FanCountThreshold` | `99999999` |
| `Skill.FanCountCoefAscArray[4].Multiply` | `1.2000000476837158` |
| `Skill.AdditionalActivateAbilityMaxCountArray[0].AdditionalActivateType` | `0` |
| `Skill.AdditionalActivateAbilityMaxCountArray[0].AdditionalActivateMaxCount` | `0` |
| `Skill.AdditionalActivateAbilityMaxCountArray[1].AdditionalActivateType` | `1` |
| `Skill.AdditionalActivateAbilityMaxCountArray[1].AdditionalActivateMaxCount` | `3` |
| `Skill.AdditionalActivateAbilityMaxCountArray[2].AdditionalActivateType` | `2` |
| `Skill.AdditionalActivateAbilityMaxCountArray[2].AdditionalActivateMaxCount` | `3` |
| `Skill.AdditionalActivateAbilityMaxCountArray[3].AdditionalActivateType` | `3` |
| `Skill.AdditionalActivateAbilityMaxCountArray[3].AdditionalActivateMaxCount` | `2` |
| `Skill.DistanceDiffTopCoefAscArray[0].DistanceDiffTopThreshold` | `20` |
| `Skill.DistanceDiffTopCoefAscArray[0].AddValue` | `0` |
| `Skill.DistanceDiffTopCoefAscArray[1].DistanceDiffTopThreshold` | `9999` |
| `Skill.DistanceDiffTopCoefAscArray[1].AddValue` | `1000` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityValueCoefAscArray1[0].BlockedSideMaxContinueTimeThreshold` | `2` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityValueCoefAscArray1[0].Multiply` | `1` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityValueCoefAscArray1[1].BlockedSideMaxContinueTimeThreshold` | `4` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityValueCoefAscArray1[1].Multiply` | `2` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityValueCoefAscArray1[2].BlockedSideMaxContinueTimeThreshold` | `6` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityValueCoefAscArray1[2].Multiply` | `3` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityValueCoefAscArray1[3].BlockedSideMaxContinueTimeThreshold` | `999` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityValueCoefAscArray1[3].Multiply` | `4` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityTimeCoefAscArray1[0].BlockedSideMaxContinueTimeThreshold` | `2` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityTimeCoefAscArray1[0].Multiply` | `1` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityTimeCoefAscArray1[1].BlockedSideMaxContinueTimeThreshold` | `4` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityTimeCoefAscArray1[1].Multiply` | `2` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityTimeCoefAscArray1[2].BlockedSideMaxContinueTimeThreshold` | `6` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityTimeCoefAscArray1[2].Multiply` | `3` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityTimeCoefAscArray1[3].BlockedSideMaxContinueTimeThreshold` | `999` |
| `Skill.BlockedSideMaxContinueTimePhaseMiddleRunAbilityTimeCoefAscArray1[3].Multiply` | `4` |
| `Skill.SpeedAbilityValueCoefAscArray1[0].SpeedThreshold` | `1700` |
| `Skill.SpeedAbilityValueCoefAscArray1[0].Multiply` | `0` |
| `Skill.SpeedAbilityValueCoefAscArray1[1].SpeedThreshold` | `1800` |
| `Skill.SpeedAbilityValueCoefAscArray1[1].Multiply` | `1` |
| `Skill.SpeedAbilityValueCoefAscArray1[2].SpeedThreshold` | `1900` |
| `Skill.SpeedAbilityValueCoefAscArray1[2].Multiply` | `2` |
| `Skill.SpeedAbilityValueCoefAscArray1[3].SpeedThreshold` | `2000` |
| `Skill.SpeedAbilityValueCoefAscArray1[3].Multiply` | `3` |
| `Skill.SpeedAbilityValueCoefAscArray1[4].SpeedThreshold` | `9999` |
| `Skill.SpeedAbilityValueCoefAscArray1[4].Multiply` | `4` |
| `Skill.SpeedAbilityValueCoefAscArray2[0].SpeedThreshold` | `1400` |
| `Skill.SpeedAbilityValueCoefAscArray2[0].Multiply` | `1` |
| `Skill.SpeedAbilityValueCoefAscArray2[1].SpeedThreshold` | `1600` |
| `Skill.SpeedAbilityValueCoefAscArray2[1].Multiply` | `2` |
| `Skill.SpeedAbilityValueCoefAscArray2[2].SpeedThreshold` | `9999` |
| `Skill.SpeedAbilityValueCoefAscArray2[2].Multiply` | `3` |
| `Popularity.Z1Add` | `1` |
| `Popularity.Z1Coef` | `0.20000000298023224` |
| `Popularity.Z2Add` | `1` |
| `Popularity.Z2Coef` | `0.20000000298023224` |
| `Popularity.RankCenterZ1Pow` | `1.5` |
| `Popularity.RankCenterZ2Pow` | `0.800000011920929` |
| `Popularity.DeviationBase` | `50` |
| `Popularity.DeviationCoef` | `10` |
| `Popularity.DeviationThreshold` | `55` |
| `Popularity.DeviationMinus` | `54` |
| `Popularity.DeviationFix` | `1` |
| `Defeat.DefeatSameRunningStylePer` | `75` |
| `Defeat.DefeatGutsOrderPer` | `33` |
| `Defeat.DefeatTemptaionCount` | `1` |
| `Defeat.DefeatFrontBlockAccumulateTime` | `1` |
| `Defeat.DefeatLoseFinishOrder` | `2` |
| `Defeat.DefeatLoseBashinDiff` | `5` |
| `Defeat.DefeatProperDistance` | `6` |
| `Defeat.DefeatProperGround` | `6` |
| `Defeat.DefeatMotivation` | `2` |
| `runMotionSpeedCoef` | `1.0499999523162842` |
| `runMotionSpeedMin` | `1.100000023841858` |
| `runMotionSpeedMax` | `1.399999976158142` |
| `runMotionSpeedBaseHorseSpeed` | `18` |
| `runMotionSpeedHorseSpeedRate` | `4.5` |
| `runMotionBlendRateMinSpeedOffset` | `1.5` |
| `runMotionBlendRateMaxSpeedOffset` | `2` |
| `laneMoveRotYThresholdPerSec` | `0.5` |
| `laneMoveRotYThresholdRotZ` | `10` |
| `laneMoveRotYPerMovedLane` | `9` |
| `laneMoveRotYMax` | `2.5` |
| `laneMoveRotYRotatePerSec` | `2.5` |
| `laneMoveRotYRotatePerSecToZero` | `2.5` |
| `laneMoveRotZThresholdPerSec` | `0.5` |
| `laneMoveRotZThresholdRotZ` | `1` |
| `laneMoveRotZPerMovedLane` | `9` |
| `laneMoveRotZMax` | `3` |
| `laneMoveRotZRotatePerSec` | `3` |
| `laneMoveRotZRotatePerSecToZero` | `3` |
| `RunMotionSpeedCoefByRunningType[0]` | `1` |
| `RunMotionSpeedCoefByRunningType[1]` | `1` |
| `RunMotionSpeedCoefByRunningType[2]` | `0.800000011920929` |
| `footDirtBaseEnergy[0]` | `0` |
| `footDirtBaseEnergy[1]` | `0.009999999776482582` |
| `footDirtBaseEnergy[2]` | `0.019999999552965164` |
| `footDirtBaseEnergy[3]` | `0.07000000029802322` |
| `lowerDirtBaseEnergy[0]` | `0` |
| `lowerDirtBaseEnergy[1]` | `0` |
| `lowerDirtBaseEnergy[2]` | `0.029999999329447746` |
| `lowerDirtBaseEnergy[3]` | `0.05000000074505806` |
| `upperDirtBaseEnergy[0]` | `0` |
| `upperDirtBaseEnergy[1]` | `0` |
| `upperDirtBaseEnergy[2]` | `0.019999999552965164` |
| `upperDirtBaseEnergy[3]` | `0.03999999910593033` |
| `dirtDistance` | `5` |
| `dirtAngle` | `25` |
| `footDirtStep[0]` | `0` |
| `footDirtStep[1]` | `20` |
| `footDirtStep[2]` | `40` |
| `footDirtStep[3]` | `60` |
| `lowerDirtStep[0]` | `0` |
| `lowerDirtStep[1]` | `20` |
| `lowerDirtStep[2]` | `40` |
| `lowerDirtStep[3]` | `60` |
| `lowerDirtStep[4]` | `80` |
| `upperDirtStep[0]` | `0` |
| `upperDirtStep[1]` | `20` |
| `upperDirtStep[2]` | `40` |
| `upperDirtStep[3]` | `60` |
| `upperDirtStep[4]` | `80` |
| `upperDirtStep[5]` | `100` |
| `EventCameraIntervalSec` | `2` |
| `EventCameraUnPlayableDistanceFromGoal` | `75` |
| `SkillCutInEventCameraUnPlayableDistanceFromGoalTeamStadium` | `75` |
| `BehindPosCheckDistanceThresholdTop` | `12.5` |
| `BehindPosCheckDistanceThreshold` | `50` |
| `BehindPosCheckLaneDistanceMeterThreshold` | `0.15000000596046448` |
| `Camera.GateInDemoPosOffsetLandscape.x` | `0` |
| `Camera.GateInDemoPosOffsetLandscape.y` | `5` |
| `Camera.GateInDemoPosOffsetLandscape.z` | `0` |
| `Camera.GateInDemoFovOffsetLandscape` | `-3` |
| `Camera.GateInDemoCut2FovLandscape` | `30` |
| `Camera.GateInDemoCut2FovLandscapeSingle` | `3.75` |
| `WaitStartDashSec` | `0.07500000298023224` |
| `Jikkyo.NearGoalDistance` | `25` |
| `Jikkyo.NearGoalInterruptDisableDistance` | `0` |
| `Jikkyo.CommentDisableDistance` | `300` |
| `Jikkyo.SilentTimeMaxSec` | `0.20000000298023224` |
| `Jikkyo.ImmidiateSuspendTime` | `0.10000000149011612` |
| `Jikkyo.NotWinRaceGroup1RaceInstanceIdArray[0]` | `100401` |
| `Jikkyo.NotWinRaceGroup1RaceInstanceIdArray[1]` | `100501` |
| `Jikkyo.NotWinRaceGroup1RaceInstanceIdArray[2]` | `100901` |
| `Jikkyo.NotWinRaceGroup1RaceInstanceIdArray[3]` | `101001` |
| `Jikkyo.NotWinRaceGroup1RaceInstanceIdArray[4]` | `101401` |
| `Jikkyo.NotWinRaceGroup1RaceInstanceIdArray[5]` | `101501` |
| `Jikkyo.WaitSecPerChar` | `0.15000000596046448` |
| `Jikkyo.WaitSecMin` | `1` |
| `Jikkyo.WaitSecMax` | `5` |
| `Jikkyo.OmitTagTime` | `5` |
| `Jikkyo.CrossTimeArray[0].SentenceCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[0].SentenceCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[0].SentenceCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[0].SentenceCrossTimeArray[3]` | `0.05999999865889549` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameBanCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameBanCrossTimeArray[1]` | `-0.05000000074505806` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameBanCrossTimeArray[2]` | `-0.05000000074505806` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameBanCrossTimeArray[3]` | `-0.009999999776482582` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[0].AfterHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[0].AfterHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[0].AfterHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[0].AfterHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameWithPostpositionalCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameWithPostpositionalCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameWithPostpositionalCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[0].BeforeHorseNameWithPostpositionalCrossTimeArray[3]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[1].SentenceCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[1].SentenceCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[1].SentenceCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[1].SentenceCrossTimeArray[3]` | `0.05999999865889549` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameBanCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameBanCrossTimeArray[1]` | `-0.05000000074505806` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameBanCrossTimeArray[2]` | `-0.05000000074505806` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameBanCrossTimeArray[3]` | `-0.009999999776482582` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[1].AfterHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[1].AfterHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[1].AfterHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[1].AfterHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameWithPostpositionalCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameWithPostpositionalCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameWithPostpositionalCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[1].BeforeHorseNameWithPostpositionalCrossTimeArray[3]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[2].SentenceCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[2].SentenceCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[2].SentenceCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[2].SentenceCrossTimeArray[3]` | `0.05999999865889549` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameBanCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameBanCrossTimeArray[1]` | `-0.05000000074505806` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameBanCrossTimeArray[2]` | `-0.05000000074505806` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameBanCrossTimeArray[3]` | `-0.009999999776482582` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[2].AfterHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[2].AfterHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[2].AfterHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[2].AfterHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameWithPostpositionalCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameWithPostpositionalCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameWithPostpositionalCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[2].BeforeHorseNameWithPostpositionalCrossTimeArray[3]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[3].SentenceCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[3].SentenceCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[3].SentenceCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[3].SentenceCrossTimeArray[3]` | `0.05999999865889549` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameBanCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameBanCrossTimeArray[1]` | `-0.05000000074505806` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameBanCrossTimeArray[2]` | `-0.05000000074505806` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameBanCrossTimeArray[3]` | `-0.009999999776482582` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[3].AfterHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[3].AfterHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[3].AfterHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[3].AfterHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameWithPostpositionalCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameWithPostpositionalCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameWithPostpositionalCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[3].BeforeHorseNameWithPostpositionalCrossTimeArray[3]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[4].SentenceCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[4].SentenceCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[4].SentenceCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[4].SentenceCrossTimeArray[3]` | `0.05999999865889549` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameBanCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameBanCrossTimeArray[1]` | `0` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameBanCrossTimeArray[2]` | `0` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameBanCrossTimeArray[3]` | `0` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[4].AfterHorseNameCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[4].AfterHorseNameCrossTimeArray[1]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[4].AfterHorseNameCrossTimeArray[2]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[4].AfterHorseNameCrossTimeArray[3]` | `0.009999999776482582` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameWithPostpositionalCrossTimeArray[0]` | `0` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameWithPostpositionalCrossTimeArray[1]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameWithPostpositionalCrossTimeArray[2]` | `0.029999999329447746` |
| `Jikkyo.CrossTimeArray[4].BeforeHorseNameWithPostpositionalCrossTimeArray[3]` | `0.029999999329447746` |
| `Sound.ResidentCrowdAisacValueArray[0]` | `0.5` |
| `Sound.ResidentCrowdAisacValueArray[1]` | `0.5` |
| `Sound.ResidentCrowdAisacValueArray[2]` | `0.5` |
| `Sound.ResidentCrowdAisacValueArray[3]` | `0.5` |
| `Sound.ResidentCrowdAisacValueArray[4]` | `0.5` |
| `Sound.ResidentCrowdAisacValueArray[5]` | `0.5` |
| `Sound.ResidentCrowdAisacValueArray[6]` | `0.6499999761581421` |
| `Sound.ResidentCrowdAisacValueArray[7]` | `0.699999988079071` |
| `Sound.ResidentCrowdAisacValueArray[8]` | `0.800000011920929` |
| `Sound.ResidentCrowdAisacValueArray[9]` | `0.8500000238418579` |
| `Sound.ResidentCrowdAisacValueArray[10]` | `0.8999999761581421` |
| `Sound.ResidentCrowdAisacInterpSecArray[0]` | `1` |
| `Sound.ResidentCrowdAisacInterpSecArray[1]` | `1` |
| `Sound.ResidentCrowdAisacInterpSecArray[2]` | `1` |
| `Sound.ResidentCrowdAisacInterpSecArray[3]` | `1` |
| `Sound.ResidentCrowdAisacInterpSecArray[4]` | `1` |
| `Sound.ResidentCrowdAisacInterpSecArray[5]` | `1` |
| `Sound.ResidentCrowdAisacInterpSecArray[6]` | `8` |
| `Sound.ResidentCrowdAisacInterpSecArray[7]` | `4.599999904632568` |
| `Sound.ResidentCrowdAisacInterpSecArray[8]` | `3.5` |
| `Sound.ResidentCrowdAisacInterpSecArray[9]` | `5` |
| `Sound.ResidentCrowdAisacInterpSecArray[10]` | `2` |
| `Sound.ResidentCrowdGoalBeforeDistance` | `50` |
| `Sound.BusDotVerticalThreshold` | `0.9700000286102295` |
| `Sound.BusFovMiddleThreshold` | `10` |
| `Sound.BusFovWideThreshold` | `30` |
| `Sound.BusDistanceMiddleForFovTightThreshold` | `16` |
| `Sound.BusDistanceFarForFovTightThreshold` | `145` |
| `Sound.BusDistanceMiddleForFovMiddleThreshold` | `16` |
| `Sound.BusDistanceFarForFovMiddleThreshold` | `50` |
| `Sound.BusDistanceMiddleForFovWideThreshold` | `16` |
| `Sound.BusDistanceFarForFovWideThreshold` | `50` |
| `Sound.BGMVolumeArray[0]` | `0` |
| `Sound.BGMVolumeArray[1]` | `1` |
| `Sound.BGMVolumeArray[2]` | `0.8999999761581421` |
| `Sound.BGMVolumeFadeSecArray[0]` | `0` |
| `Sound.BGMVolumeFadeSecArray[1]` | `1` |
| `Sound.BGMVolumeFadeSecArray[2]` | `10` |

## Source constants

Exact declarations from the source, including implementation/version constants.

### [activation_conditions.rs](../src/activation_conditions.rs)

```rust
const CORNER_RANDOM_QUAD_SKILLS: &[&str] = &[
    "200331", "200332", "200333", "200341", "200342", "200343", "200351", "200352", "200353",
    "200971", "200972", "201041", "201042", "201111", "201112", "201181", "201182",
    "201251", "201252", "201321", "201322", "201391", "201392", "201461", "201462",
];
const PHASE_FUDGE_SKILLS: &[&str] = &["100591", "900591", "110261", "910261", "110191", "910191", "120451", "920451", "101502121"];
const DIRT_GRADE_TRACK_IDS: &[i32] = &[10101, 10103, 10104, 10105];
const FURLONG_METERS: f64 = 201.168;
const TIGHT_TRACK_IDS: &[i32] = &[10001, 10002, 10004, 10010, 10103, 10104];
```

### [course_data.rs](../src/course_data.rs)

```rust
pub const SURFACE_TURF: i32 = 1;
pub const SURFACE_DIRT: i32 = 2;
```

### [hp_policy.rs](../src/hp_policy.rs)

```rust
const HP_STRATEGY_COEFFICIENT: [f64; 6] = [0.0, 0.95, 0.89, 1.0, 0.995, 0.86];
const HP_CONSUMPTION_GROUND_MODIFIER: [[f64; 5]; 3] = [
    [0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 1.0, 1.02, 1.02],
    [0.0, 1.0, 1.0, 1.01, 1.02],
];
```

### [lane_selection.rs](../src/lane_selection.rs)

```rust
const LANE: f64 = 1.0 / 18.0;
```

### [main.rs](../src/main.rs)

```rust
const DT: f64 = 1.0 / 15.0;
const MAX_RACE_SECONDS: f64 = 400.0;
const COURSE_DATA_JSON: &str = include_str!("../data/course_data.json");
const SKILL_DATA_JSON: &str = include_str!("../data/skill_data.json");
const DENSE_LEAD_FRAMES: usize = 17;
const SPARSE_STRIDE: usize = 16;
```

### [race_field.rs](../src/race_field.rs)

```rust
pub const HORSE_LANE: f64 = 1.0 / 18.0;
const PACEMAKER_GROUPS: &[&[Strategy]] = &[&[Strategy::Nige, Strategy::Oonige], &[Strategy::Senkou], &[Strategy::Sasi], &[Strategy::Oikomi]];
```

### [race_solver.rs](../src/race_solver.rs)

```rust
const SPOT_STRUGGLE_LANE_GAP: f64 = 0.165;
const DUEL_LANE_GAP: f64 = 0.25;
const SPOT_STRUGGLE_CHECK_END_SECTION: f64 = 6.0;
const SPOT_STRUGGLE_END_SECTION: f64 = 8.0;
const SPEED_STRATEGY_PHASE_COEFFICIENT: [[f64; 3]; 6] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.98, 0.962],
    [0.978, 0.991, 0.975],
    [0.938, 0.998, 0.994],
    [0.931, 1.0, 1.0],
    [1.063, 0.962, 0.95],
];
const SPEED_DISTANCE_PROFICIENCY_MODIFIER: [f64; 8] = [1.05, 1.0, 0.9, 0.8, 0.6, 0.4, 0.2, 0.1];
const ACCEL_STRATEGY_PHASE_COEFFICIENT: [[f64; 3]; 6] = [
    [0.0, 0.0, 0.0],
    [1.0, 1.0, 0.996],
    [0.985, 1.0, 0.996],
    [0.975, 1.0, 1.0],
    [0.945, 1.0, 0.997],
    [1.17, 0.94, 0.956],
];
const ACCEL_GROUND_TYPE_PROFICIENCY_MODIFIER: [f64; 8] = [1.05, 1.0, 0.9, 0.8, 0.7, 0.5, 0.3, 0.1];
const ACCEL_DISTANCE_PROFICIENCY_MODIFIER: [f64; 8] = [1.0, 1.0, 1.0, 1.0, 1.0, 0.6, 0.5, 0.4];
const BASE_ACCEL: f64 = 0.0006;
const UPHILL_BASE_ACCEL: f64 = 0.0004;
const PHASE_DECELERATION: [f64; 3] = [-1.2, -0.8, -1.0];
const STRATEGY_PROFICIENCY_MODIFIER_APPROX: [f64; 8] = [1.1, 1.0, 0.85, 0.75, 0.6, 0.4, 0.2, 0.1];
const SECURE_LEAD_COEF_APPROX: f64 = 3.0;
const POSITION_KEEP_BASE_MIN_THRESHOLD: [f64; 6] = [0.0, 0.0, 3.0, 6.5, 7.5, f64::NAN];
const POSITION_KEEP_BASE_MAX_THRESHOLD: [f64; 6] = [0.0, 0.0, 5.0, 7.0, 8.0, f64::NAN];
```

### [race_solver_builder.rs](../src/race_solver_builder.rs)

```rust
const GROUND_SPEED_MODIFIER: [[f64; 5]; 3] = [[0.0; 5], [0.0, 0.0, 0.0, 0.0, -50.0], [0.0, 0.0, 0.0, 0.0, -50.0]];
const GROUND_POWER_MODIFIER: [[f64; 5]; 3] = [[0.0; 5], [0.0, 0.0, -50.0, -50.0, -50.0], [0.0, -100.0, -50.0, -100.0, -100.0]];
const STRATEGY_PROFICIENCY_MODIFIER: [f64; 8] = [1.1, 1.0, 0.85, 0.75, 0.6, 0.4, 0.2, 0.1];
const ASITAME_STRATEGY_DISTANCE_COEFFICIENT: [[f64; 6]; 5] = [
    [0.0; 6],
    [0.0, 1.0, 0.7, 0.75, 0.7, 1.0],
    [0.0, 1.0, 0.8, 0.7, 0.75, 1.0],
    [0.0, 1.0, 0.9, 0.875, 0.86, 1.0],
    [0.0, 1.0, 0.9, 1.0, 0.9, 1.0],
];
const ASITAME_BASE_MODIFIER: f64 = 0.00875;
const K: f64 = 0.0185;
const SKILL_TARGET_SELF: i32 = 1;
const SKILL_TARGET_ALL: i32 = 2;
const LEVEL_MULTIPLIER_TARGET_SPEED: [f64; 11] = [1.00, 1.00, 1.01, 1.04, 1.07, 1.10, 1.13, 1.16, 1.19, 1.22, 1.25];
const LEVEL_MULTIPLIER_ACCEL: [f64; 11] = [1.00, 1.00, 1.02, 1.04, 1.06, 1.08, 1.10, 1.125, 1.15, 1.175, 1.20];
const LEVEL_MULTIPLIER_STAT: [f64; 11] = [1.00, 1.00, 1.01, 1.02, 1.03, 1.04, 1.05, 1.06, 1.07, 1.08, 1.10];
const LEVEL_MULTIPLIER_OTHER: [f64; 11] = [1.00, 1.00, 1.02, 1.04, 1.06, 1.08, 1.10, 1.12, 1.14, 1.16, 1.18];
```

### [random_gen.rs](../src/random_gen.rs)

```rust
const MASK_HI: u64 = 0x03FF_FFFF;
const MASK_LO: u64 = 0x07FF_FFFF;
const EXP: u64 = 0x0800_0000;
const MANT: f64 = 9_007_199_254_740_992.0;
```

### [retail_lane.rs](../src/retail_lane.rs)

```rust
const EDGE: f32 = 0.055555559694767_f32;
const STRAIGHT: f32 = 0.0555555559694767_f32;
const CORRIDOR: f32 = 0.04444444924592972_f32;
const CROWD: f32 = 0.1111111119389534_f32;
```
