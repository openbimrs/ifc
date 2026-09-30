//! Inline STEP fixtures shared by the coordinate-operation tests.
//!
//! Each test states only the entities it is about; `BASE` supplies units,
//! the model context, a projected and a geographic CRS. Kept inline rather
//! than under `test/fixtures/` because every case differs by one entity.

use ifc_model::{Codec, Model};
use ifc_step::StepCodec;

/// `#1` metre, `#2` radian, `#3` degree, `#5` placement, `#7` the model
/// context, `#8` a body sub-context, `#50` `EPSG:25832` in metres.
pub const BASE: &str = "\
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#3=IFCCONVERSIONBASEDUNIT(#9,.PLANEANGLEUNIT.,'DEGREE',#10);
#4=IFCCARTESIANPOINT((0.,0.,0.));
#5=IFCAXIS2PLACEMENT3D(#4,$,$);
#7=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-6,#5,$);
#8=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#7,$,.MODEL_VIEW.,$);
#9=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#10=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.0174532925199433),#2);
#50=IFCPROJECTEDCRS('EPSG:25832','ETRS89 / UTM zone 32N','ETRS89','DHHN2016','UTM','32N',#1);
";

/// IFC4X3 only: `#70` a geographic CRS in degrees and metres.
pub const GEOGRAPHIC: &str = "\
#70=IFCGEOGRAPHICCRS('EPSG:4979','WGS 84','WGS84','Greenwich',#3,#1);
";

/// Parse `data` (entity lines) under a `FILE_SCHEMA` of `schema`.
pub fn step(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');\n\
         FILE_NAME('georef.ifc','2026-09-30T00:00:00',('openbimrs'),('openbimrs'),'','','');\n\
         FILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n{data}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("inline fixture parses")
}
