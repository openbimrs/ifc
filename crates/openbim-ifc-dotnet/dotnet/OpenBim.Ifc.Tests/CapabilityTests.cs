// The #244 surface: lenient reads, the header, validation, ifcXML and the
// reachability lint, as smoke.c's `capabilities()` checks them.

using System.Linq;
using Xunit;

namespace OpenBim.Ifc.Tests;

public class CapabilityTests
{
    [Fact]
    public void BeyondTheRecordModel()
    {
        var data = Fixtures.Bytes(Fixtures.Damaged);
        // docs:snippet dotnet-beyond-records
        // A damaged export: skip what cannot be read, and say what was skipped.
        using var model = IfcModel.Parse(data, ParseOptions.Lenient);
        var skipped = model.Diagnostics; // one message per recovery

        var header = model.Header; // Header { Name = ..., Author = [...], Schema = [...] }
        model.Header = header with { Author = new EquatableList<string>(new[] { "Reviewer" }) };

        var report = model.Validate(); // ValidationReport { Conformant = ..., Findings = [...] }
        var errors = report.Findings.Where(finding => finding.Severity == "error").ToList();

        var xml = model.WriteIfcXml(); // lossless ifcXML; or xsdProfile: "IFC4"
        using var fromXml = IfcModel.ParseIfcXml(xml);
        // docs:end

        Assert.Single(skipped);
        Assert.Equal(new[] { "Reviewer" }, model.Header.Author);
        Assert.NotEmpty(errors);
        Assert.Equal(2, fromXml.Count);
    }

    [Fact]
    public void AStrictReadRefusesWhatALenientOneSkips()
    {
        var data = Fixtures.Bytes(Fixtures.Damaged);
        Assert.Equal("parse", Assert.Throws<IfcException>(() => IfcModel.Parse(data)).Code);
        using var model = IfcModel.Parse(data, new ParseOptions(OnMalformed.Skip, CheckReferences: true));
        Assert.True(model.Diagnostics.Count >= 2, string.Join("\n", model.Diagnostics));
    }

    [Fact]
    public void TheHeaderIsReadAndReplacedWhole()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Damaged), ParseOptions.Lenient);
        var header = model.Header;
        Assert.Equal("d.ifc", header.Name);
        Assert.Equal("2;1", header.ImplementationLevel);
        Assert.Equal(new[] { "Ann" }, header.Author);
        Assert.Equal(new[] { "IFC4" }, header.Schema);

        model.Header = header; // a no-op replacement is accepted
        Assert.Equal(header, model.Header);
        model.Header = header with { Name = "renamed.ifc" };
        using var again = IfcModel.Parse(model.Write());
        Assert.Equal("renamed.ifc", again.Header.Name);
    }

    [Fact]
    public void ValidationReportsTheDanglingReference()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.Damaged), ParseOptions.Lenient);
        var report = model.Validate();
        Assert.False(report.Conformant);
        Assert.False(report.Truncated);
        Assert.True(report.Errors >= 1);
        Assert.Equal(report.Errors + report.EvaluationErrors + report.Warnings + report.Unsupported, report.Findings.Count);
        Assert.Contains(report.Findings, finding => finding.Entity == 3UL && finding.Severity == "error");

        var capped = model.Validate(maxFindings: 1);
        Assert.Single(capped.Findings);
        Assert.Equal(new DanglingReference(3, 9), Assert.Single(model.DanglingReferences()));
    }

    [Fact]
    public void IfcXmlRoundTripsAndRefusesAnUnknownProfile()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        using var fromXml = IfcModel.ParseIfcXml(model.WriteIfcXml());
        Assert.Equal(model.Attributes(1), fromXml.Attributes(1));
        Assert.Equal(model.Attributes(2), fromXml.Attributes(2));

        var error = Assert.Throws<IfcException>(() => model.WriteIfcXml("IFC2X3"));
        Assert.Equal("unsupported-profile", error.Code);
        Assert.Equal(IfcStatus.UnsupportedProfile, error.Status);
        Assert.Equal("unsupported-profile",
            Assert.Throws<IfcException>(() => IfcModel.ParseIfcXml(new byte[] { 0x3c }, "IFC2X3")).Code);
    }

    [Fact]
    public void AWallWithoutRepresentationIsNotUnreachable()
    {
        using var model = IfcModel.Parse(Fixtures.Bytes(Fixtures.File));
        Assert.Empty(model.UnreachableProducts());
    }
}
