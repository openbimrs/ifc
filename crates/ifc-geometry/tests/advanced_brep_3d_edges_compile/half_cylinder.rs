//! Planar and curved faces sharing straight edges close (#528,
//! axiolid/kernel#336): a D-shaped half cylinder.

use super::*;

/// A D-shaped half cylinder (`HALF`), metres: the half `y >= 0` of a
/// cylinder of radius `r` and height `h` about the Z axis, as an advanced
/// B-rep of four faces. Its `IfcCylindricalSurface` wall is bounded by the
/// half-circle rims (`IfcCircle` edges from `(r, 0, z)` to `(-r, 0, z)`) and
/// two `IfcLine` rulings at `x = r` and `x = -r`; the planar face `y = 0`
/// shares those rulings and is closed by two `IfcPolyline` diameters, which
/// bound the half-disk caps with the rims.
fn half_cylinder(schema: &str, r: f64, h: f64) -> Model {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [DesignTransferView]'),'2;1');
FILE_NAME('half_cylinder.ifc','2026-10-20T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('{schema}'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCUNITASSIGNMENT((#1));
#3=IFCCARTESIANPOINT((0.,0.,0.));
#4=IFCAXIS2PLACEMENT3D(#3,$,$);
#5=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#6=IFCPROJECT('3Wq7nHx2X0Ef8Kw$3Jd9aE',$,'Half cylinder',$,$,$,$,(#5),#2);
#10=IFCDIRECTION((0.,0.,1.));
#11=IFCDIRECTION((1.,0.,0.));
#12=IFCDIRECTION((0.,0.,-1.));
#13=IFCDIRECTION((0.,-1.,0.));
#15=IFCCARTESIANPOINT((0.,0.,{h:?}));
#16=IFCAXIS2PLACEMENT3D(#3,#10,#11);
#17=IFCAXIS2PLACEMENT3D(#15,#10,#11);
#18=IFCCYLINDRICALSURFACE(#16,{r:?});
#19=IFCAXIS2PLACEMENT3D(#3,#12,#11);
#20=IFCPLANE(#19);
#21=IFCPLANE(#17);
#22=IFCAXIS2PLACEMENT3D(#3,#13,#11);
#23=IFCPLANE(#22);
#30=IFCCARTESIANPOINT(({r:?},0.,0.));
#31=IFCCARTESIANPOINT(({mr:?},0.,0.));
#32=IFCCARTESIANPOINT(({r:?},0.,{h:?}));
#33=IFCCARTESIANPOINT(({mr:?},0.,{h:?}));
#34=IFCVERTEXPOINT(#30);
#35=IFCVERTEXPOINT(#31);
#36=IFCVERTEXPOINT(#32);
#37=IFCVERTEXPOINT(#33);
#40=IFCCIRCLE(#16,{r:?});
#41=IFCCIRCLE(#17,{r:?});
#42=IFCVECTOR(#10,{h:?});
#43=IFCLINE(#30,#42);
#44=IFCLINE(#31,#42);
#45=IFCPOLYLINE((#31,#30));
#46=IFCPOLYLINE((#33,#32));
#50=IFCEDGECURVE(#34,#35,#40,.T.);
#51=IFCEDGECURVE(#36,#37,#41,.T.);
#52=IFCEDGECURVE(#34,#36,#43,.T.);
#53=IFCEDGECURVE(#35,#37,#44,.T.);
#54=IFCEDGECURVE(#35,#34,#45,.T.);
#55=IFCEDGECURVE(#37,#36,#46,.T.);
#60=IFCORIENTEDEDGE(*,*,#50,.T.);
#61=IFCORIENTEDEDGE(*,*,#53,.T.);
#62=IFCORIENTEDEDGE(*,*,#51,.F.);
#63=IFCORIENTEDEDGE(*,*,#52,.F.);
#64=IFCEDGELOOP((#60,#61,#62,#63));
#65=IFCFACEOUTERBOUND(#64,.T.);
#66=IFCADVANCEDFACE((#65),#18,.T.);
#70=IFCORIENTEDEDGE(*,*,#54,.T.);
#71=IFCORIENTEDEDGE(*,*,#52,.T.);
#72=IFCORIENTEDEDGE(*,*,#55,.F.);
#73=IFCORIENTEDEDGE(*,*,#53,.F.);
#74=IFCEDGELOOP((#70,#71,#72,#73));
#75=IFCFACEOUTERBOUND(#74,.T.);
#76=IFCADVANCEDFACE((#75),#23,.T.);
#80=IFCORIENTEDEDGE(*,*,#50,.F.);
#81=IFCORIENTEDEDGE(*,*,#54,.F.);
#82=IFCEDGELOOP((#80,#81));
#83=IFCFACEOUTERBOUND(#82,.T.);
#84=IFCADVANCEDFACE((#83),#20,.T.);
#85=IFCORIENTEDEDGE(*,*,#51,.T.);
#86=IFCORIENTEDEDGE(*,*,#55,.T.);
#87=IFCEDGELOOP((#85,#86));
#88=IFCFACEOUTERBOUND(#87,.T.);
#89=IFCADVANCEDFACE((#88),#21,.T.);
#100=IFCCLOSEDSHELL((#66,#76,#84,#89));
#101=IFCADVANCEDBREP(#100);
#102=IFCSHAPEREPRESENTATION(#5,'Body','AdvancedBrep',(#101));
#103=IFCPRODUCTDEFINITIONSHAPE($,$,(#102));
#104=IFCLOCALPLACEMENT($,#4);
#105=IFCBUILDINGELEMENTPROXY('1Hq7nHx2X0Ef8Kw$3Jd9aF',$,'HALF',$,$,#104,#103,$,$);
ENDSEC;
END-ISO-10303-21;
",
        mr = -r,
    );
    parse(&text)
}

/// The D-shaped half cylinder closes (axiolid/kernel#336; #528): its wall
/// and its planar face take the same samples of the rulings they share, so
/// the mesh has no T-junction there. It holds `pi r^2 h / 2` less its
/// chorded rims, by no more than its surface area times the 1 mm budget,
/// and its report is certified within the request.
#[test]
fn a_d_shaped_half_cylinder_compiles_watertight_to_its_volume() {
    let (r, h) = (0.5_f64, 2.0_f64);
    let exact = PI * r * r * h / 2.0;
    let area = PI * r * h + 2.0 * r * h + PI * r * r;
    for schema in ["IFC4", "IFC4X3"] {
        let model = half_cylinder(schema, r, h);
        let mesh = solid(&model, "HALF");
        let volume = signed_volume(&mesh);
        assert!(
            volume <= exact && exact - volume <= area * 1e-3,
            "{schema}: volume {volume}, exact {exact}"
        );
        assert_bounds(schema, &mesh, [-r, 0.0, 0.0], [r, r, h], 1e-3);
        let report = report(&model, "HALF");
        assert!(report.bound.is_some(), "{schema}: certified: {report:?}");
        assert!(report.meets_requested(), "{schema}: {report:?}");
        assert_eq!(report.unbounded().count(), 0, "{schema}: {report:?}");
        let details: Vec<&str> = report.contributions.iter().map(|c| c.detail).collect();
        assert!(
            details.contains(&"cylinder") && details.contains(&"plane"),
            "{schema}: {report:?}"
        );
    }
}
