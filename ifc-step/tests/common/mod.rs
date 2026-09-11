use std::fmt::Write as _;

pub fn synthesize(walls: usize) -> String {
    let mut s = String::with_capacity(walls * 512);
    s.push_str("ISO-10303-21;\nHEADER;\n");
    s.push_str("FILE_DESCRIPTION((''),'2;1');\n");
    s.push_str("FILE_NAME('scale.ifc','2026-01-01T00:00:00',(''),(''),'','','');\n");
    s.push_str("FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n");
    s.push_str("#1=IFCPERSON($,'A',$,$,$,$,$,$);\n");
    s.push_str("#2=IFCORGANIZATION($,'O',$,$,$);\n");
    s.push_str("#3=IFCPERSONANDORGANIZATION(#1,#2,$);\n");
    s.push_str("#4=IFCAPPLICATION(#2,'1','app','app');\n");
    s.push_str("#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);\n");
    s.push_str("#6=IFCCARTESIANPOINT((0.,0.,0.));\n");
    s.push_str("#7=IFCAXIS2PLACEMENT3D(#6,$,$);\n");
    s.push_str("#8=IFCLOCALPLACEMENT($,#7);\n");
    let mut id = 9usize;
    for i in 0..walls {
        let p = id;
        let _ = writeln!(s, "#{}=IFCCARTESIANPOINT(({}.,{}.,0.));", id, i * 3, i * 2);
        id += 1;
        let a = id;
        let _ = writeln!(s, "#{id}=IFCAXIS2PLACEMENT3D(#{p},$,$);");
        id += 1;
        let lp = id;
        let _ = writeln!(s, "#{id}=IFCLOCALPLACEMENT(#8,#{a});");
        id += 1;
        let pl = id;
        let _ = writeln!(s, "#{id}=IFCPOLYLINE((#{p},#{p}));");
        id += 1;
        let sr = id;
        let _ = writeln!(
            s,
            "#{id}=IFCSHAPEREPRESENTATION($,'Axis','Curve2D',(#{pl}));"
        );
        id += 1;
        let pd = id;
        let _ = writeln!(s, "#{id}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{sr}));");
        id += 1;
        let w = id;
        let _ = writeln!(
            s,
            "#{id}=IFCWALL('{i:022}',#5,'Wall-{i}',$,$,#{lp},#{pd},$,$);"
        );
        id += 1;
        let pv = id;
        let _ = writeln!(
            s,
            "#{id}=IFCPROPERTYSINGLEVALUE('P{i}',$,IFCLABEL('v{i}'),$);"
        );
        id += 1;
        let ps = id;
        let _ = writeln!(
            s,
            "#{}=IFCPROPERTYSET('{:022}',#5,'Pset_{}',$,(#{}));",
            id,
            i + 1_000_000,
            i,
            pv
        );
        id += 1;
        let _ = writeln!(
            s,
            "#{}=IFCRELDEFINESBYPROPERTIES('{:022}',#5,$,$,(#{}),#{});",
            id,
            i + 2_000_000,
            w,
            ps
        );
        id += 1;
    }
    s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    s
}
