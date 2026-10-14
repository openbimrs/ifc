// Schema-checked entity creation (#330): a model built from nothing that
// validates clean and round-trips through STEP and ifcXML, a refused batch
// that leaves the model byte-identical, and the shared codes, as smoke.c's
// `authoring()` checks them.

using System.Collections.Generic;
using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class AuthoringTests
{
    private static IfcModel EmptyModel(string schema = "IFC4")
    {
        var model = new IfcModel();
        model.Header = model.Header with { Schema = new EquatableList<string>(new[] { schema }) };
        return model;
    }

    private static Dictionary<string, Value> Attributes(params (string Name, Value Value)[] pairs) =>
        pairs.ToDictionary(pair => pair.Name, pair => pair.Value);

    private static Value Ref(ulong id) => new Value.Ref(id);

    /// <summary>Units, a context, project, site, building and storey, a wall type, and a placed wall contained in the storey and typed.</summary>
    private static IReadOnlyList<ulong?> Building(IfcModel model)
    {
        static ulong H(int index) => IfcModel.Handle(index);
        return model.Author(new[]
        {
            AuthorOp.Create("IfcSIUnit", Attributes(("UnitType", new Value.Enum("LENGTHUNIT")), ("Name", new Value.Enum("METRE")))),
            AuthorOp.Create("IfcUnitAssignment", Attributes(("Units", new Value.List(Ref(H(0)))))),
            AuthorOp.Create("IfcCartesianPoint", Attributes(("Coordinates", new Value.List(new Value.Real(0), new Value.Real(0), new Value.Real(0))))),
            AuthorOp.Create("IfcAxis2Placement3D", Attributes(("Location", Ref(H(2))))),
            AuthorOp.Create("IfcGeometricRepresentationContext", Attributes(
                ("ContextType", new Value.Text("Model")),
                ("CoordinateSpaceDimension", new Value.Integer(3)),
                ("Precision", new Value.Real(1e-5)),
                ("WorldCoordinateSystem", Ref(H(3))))),
            AuthorOp.Project(Attributes(
                ("Name", new Value.Text("Demo")),
                ("UnitsInContext", Ref(H(1))),
                ("RepresentationContexts", new Value.List(Ref(H(4)))))),
            AuthorOp.Placement(),
            AuthorOp.Spatial("IfcSite", H(5), placement: H(6)),
            AuthorOp.Placement(relativeTo: H(6)),
            AuthorOp.Spatial("IfcBuilding", H(7), placement: H(8)),
            AuthorOp.Placement(relativeTo: H(8)),
            AuthorOp.Spatial("IfcBuildingStorey", H(9), Attributes(("Name", new Value.Text("Level 0"))), placement: H(10)),
            AuthorOp.TypeObject("IfcWallType", Attributes(("PredefinedType", new Value.Enum("STANDARD")))),
            AuthorOp.Placement(relativeTo: H(10), location: (1, 2, 0), axis: (0, 0, 1), refDirection: (1, 0, 0)),
            AuthorOp.Product("IfcWall", Attributes(("Name", new Value.Text("Wall"))), container: H(11), placement: H(13), typeObject: H(12)),
        });
    }

    [Fact]
    public void DocumentedExample()
    {
        using var model = EmptyModel("IFC4");
        // docs:snippet dotnet-authoring
        // IfcModel.Handle(i): the entity operation i of the batch produces.
        var ids = model.Author(new[]
        {
            AuthorOp.Project(new Dictionary<string, Value> { ["Name"] = new Value.Text("Demo") }), // 0
            AuthorOp.Placement(), // 1: at the origin
            AuthorOp.Spatial("IfcSite", IfcModel.Handle(0), placement: IfcModel.Handle(1)), // 2
            AuthorOp.Spatial("IfcBuilding", IfcModel.Handle(2)), // 3
            AuthorOp.Placement(relativeTo: IfcModel.Handle(1)), // 4
            AuthorOp.Spatial("IfcBuildingStorey", IfcModel.Handle(3), placement: IfcModel.Handle(4)), // 5
            AuthorOp.TypeObject("IfcWallType", new Dictionary<string, Value> { ["PredefinedType"] = new Value.Enum("STANDARD") }), // 6
            AuthorOp.Placement(relativeTo: IfcModel.Handle(4), location: (1, 2, 0)), // 7
            AuthorOp.Product(
                "IfcWall",
                new Dictionary<string, Value> { ["Name"] = new Value.Text("Wall") },
                container: IfcModel.Handle(5), // IfcRelContainedInSpatialStructure
                placement: IfcModel.Handle(7),
                typeObject: IfcModel.Handle(6)), // IfcRelDefinesByType
        });
        var wall = ids[8]!.Value; // every IfcRoot got a GlobalId
        // docs:end
        Assert.Equal("IFCWALL", model.TypeOf(wall));
        Assert.Equal(22, ((Value.Text)model.AttributeByName(wall, "GlobalId")).Value.Length);
        var report = model.Validate();
        Assert.Equal(0L, report.Errors + report.EvaluationErrors);
    }

    [Fact]
    public void AModelBuiltFromNothingValidatesAndRoundTrips()
    {
        using var model = EmptyModel();
        var wall = Building(model)[14]!.Value;
        // A property set through the #316 call.
        model.SetProperty(wall, "ACME_WallData", "Mark", new Value.Typed("IFCLABEL", new Value.Text("W-01")));
        var report = model.Validate();
        Assert.True(report.Errors + report.EvaluationErrors == 0, string.Join("\n", report.Findings.Select(f => f.Message)));

        var step = model.Write();
        using (var again = IfcModel.Parse(step))
        {
            Assert.Equal(step, again.Write());
        }
        foreach (var profile in new[] { null, "IFC4" })
        {
            using var back = IfcModel.ParseIfcXml(model.WriteIfcXml(profile), profile);
            Assert.Equal(step, back.Write());
        }
    }

    [Fact]
    public void ARefusedBatchLeavesTheModelByteIdentical()
    {
        using var model = EmptyModel();
        var ids = Building(model);
        var before = model.Write();
        var error = Assert.Throws<IfcException>(() => model.Author(new[]
        {
            AuthorOp.Product("IfcWall", container: ids[11]),
            AuthorOp.Contain(ids[11]!.Value, new[] { ids[14]!.Value }),
        }));
        Assert.Equal("invalid-model", error.Code);
        Assert.Equal(IfcStatus.InvalidModel, error.Status);
        Assert.Equal(before, model.Write());
    }

    [Fact]
    public void SingleCalls()
    {
        using var model = EmptyModel();
        var ids = Building(model);
        var proxy = model.CreateEntity("IfcBuildingElementProxy", Attributes(("Name", new Value.Text("Proxy"))));
        Assert.Equal("IFCBUILDINGELEMENTPROXY", model.TypeOf(proxy));
        var blocked = Assert.Throws<IfcException>(() => model.RemoveWithRelationships(ids[10]!.Value));
        Assert.Equal("still-referenced", blocked.Code);
        Assert.Equal(IfcStatus.StillReferenced, blocked.Status);
        model.RemoveWithRelationships(ids[14]!.Value);
        Assert.Empty(model.IdsOfType("IfcRelContainedInSpatialStructure"));
        Assert.Empty(model.DanglingReferences());
    }

    [Fact]
    public void RefusalsCarryTheSharedCodes()
    {
        using var model = EmptyModel();
        var ids = Building(model);
        var before = model.Write();
        var cases = new (string Code, IfcStatus Status, AuthorOp Op)[]
        {
            ("unsupported-schema", IfcStatus.UnsupportedSchema, AuthorOp.Create("IfcWal")),
            ("wrong-entity-type", IfcStatus.WrongEntityType, AuthorOp.Create("IfcElement")),
            ("unknown-attribute", IfcStatus.UnknownAttribute, AuthorOp.Create("IfcWall", Attributes(("Nmae", new Value.Text("x"))))),
            ("missing-attribute", IfcStatus.MissingAttribute, AuthorOp.Create("IfcWallType")),
            ("invalid-value", IfcStatus.InvalidValue, AuthorOp.Create("IfcWall", Attributes(("Name", new Value.Integer(3))))),
            ("missing-reference", IfcStatus.MissingReference, AuthorOp.Create("IfcWall", Attributes(("ObjectPlacement", Ref(99999))))),
            ("missing-entity", IfcStatus.MissingEntity, AuthorOp.Edit(99999, Attributes())),
            ("derived-attribute", IfcStatus.DerivedAttribute, AuthorOp.Edit(ids[0]!.Value, Attributes(("Dimensions", new Value.Null())))),
            ("invalid-model", IfcStatus.InvalidModel, AuthorOp.Project()),
            ("still-referenced", IfcStatus.StillReferenced, AuthorOp.Remove(ids[10]!.Value)),
            ("invalid-value", IfcStatus.InvalidValue, AuthorOp.Placement(axis: (0, 0, 1))),
            ("invalid-value", IfcStatus.InvalidValue, AuthorOp.Create("IfcWall", Attributes(("ObjectPlacement", Ref(IfcModel.Handle(0)))))),
        };
        foreach (var (code, status, op) in cases)
        {
            var error = Assert.Throws<IfcException>(() => model.Author(new[] { op }));
            Assert.Equal(code, error.Code);
            Assert.Equal(status, error.Status);
            Assert.Equal(before, model.Write());
        }
        using var ifc2x3 = EmptyModel("IFC2X3");
        Assert.Equal("missing-attribute", Assert.Throws<IfcException>(() => ifc2x3.Author(new[] { AuthorOp.Project() })).Code);
    }
    [Fact]
    public void AssignTypePropagatesAndReplaces()
    {
        // #480: map the type's representation, derive the layer set usage,
        // sync PredefinedType, and replace the previous type and container.
        using var model = EmptyModel();
        var ids = Building(model);
        ulong context = ids[4]!.Value, building = ids[9]!.Value, wall = ids[14]!.Value;
        static ulong H(int index) => IfcModel.Handle(index);
        var added = model.Author(new[]
        {
            AuthorOp.Create("IfcCartesianPoint", Attributes(("Coordinates", new Value.List(new Value.Real(0), new Value.Real(0), new Value.Real(0))))), // 0
            AuthorOp.Create("IfcAxis2Placement3D", Attributes(("Location", Ref(H(0))))), // 1
            AuthorOp.Create("IfcBoundingBox", Attributes(
                ("Corner", Ref(H(0))), ("XDim", new Value.Real(5)), ("YDim", new Value.Real(0.2)), ("ZDim", new Value.Real(3)))), // 2
            AuthorOp.Create("IfcShapeRepresentation", Attributes(
                ("ContextOfItems", Ref(context)),
                ("RepresentationIdentifier", new Value.Text("Body")),
                ("RepresentationType", new Value.Text("BoundingBox")),
                ("Items", new Value.List(Ref(H(2)))))), // 3
            AuthorOp.Create("IfcRepresentationMap", Attributes(("MappingOrigin", Ref(H(1))), ("MappedRepresentation", Ref(H(3))))), // 4
            AuthorOp.Create("IfcMaterial", Attributes(("Name", new Value.Text("Concrete")))), // 5
            AuthorOp.Create("IfcMaterialLayer", Attributes(("Material", Ref(H(5))), ("LayerThickness", new Value.Real(0.2)))), // 6
            AuthorOp.Create("IfcMaterialLayerSet", Attributes(("MaterialLayers", new Value.List(Ref(H(6)))))), // 7
            AuthorOp.TypeObject("IfcWallType", Attributes(
                ("PredefinedType", new Value.Enum("SOLIDWALL")),
                ("RepresentationMaps", new Value.List(Ref(H(4)))))), // 8
            AuthorOp.Create("IfcRelAssociatesMaterial", Attributes(
                ("RelatedObjects", new Value.List(Ref(H(8)))), ("RelatingMaterial", Ref(H(7))))), // 9
            AuthorOp.Spatial("IfcBuildingStorey", building, Attributes(("Name", new Value.Text("Level 1")))), // 10
        });
        ulong newType = added[8]!.Value, level1 = added[10]!.Value;
        Assert.Equal("invalid-model", Assert.Throws<IfcException>(() => model.Author(new[] { AuthorOp.AssignType(newType, new[] { wall }) })).Code);
        Assert.Equal("invalid-model", Assert.Throws<IfcException>(() => model.Author(new[] { AuthorOp.Contain(level1, new[] { wall }) })).Code);
        model.Author(new[]
        {
            AuthorOp.AssignType(
                newType,
                new[] { wall },
                mapRepresentations: true,
                materialUsage: true,
                predefinedType: PredefinedTypeMode.Sync,
                replace: true),
            AuthorOp.Contain(level1, new[] { wall }, replace: true),
        });
        Assert.Equal(newType, model.TypeObject(wall));
        var shape = ((Value.Ref)model.AttributeByName(wall, "Representation")).Id;
        var representation = ((Value.Ref)((Value.List)model.AttributeByName(shape, "Representations")).Items[0]).Id;
        Assert.Equal(new Value.Text("MappedRepresentation"), model.AttributeByName(representation, "RepresentationType"));
        var report = model.Validate();
        Assert.Equal(0L, report.Errors + report.EvaluationErrors);
    }
}
