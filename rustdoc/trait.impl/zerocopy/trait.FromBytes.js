(function() {
    var implementors = Object.fromEntries([["svsm",[["impl FromBytes for <a class=\"struct\" href=\"svsm/acpi/tables/struct.RSDPDesc.html\" title=\"struct svsm::acpi::tables::RSDPDesc\">RSDPDesc</a><div class=\"where\">where\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">8</a>]: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">6</a>]: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/acpi/tables/struct.RawACPITableHeader.html\" title=\"struct svsm::acpi::tables::RawACPITableHeader\">RawACPITableHeader</a><div class=\"where\">where\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">4</a>]: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">6</a>]: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">8</a>]: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/acpi/tables/struct.RawMADTEntryHeader.html\" title=\"struct svsm::acpi::tables::RawMADTEntryHeader\">RawMADTEntryHeader</a><div class=\"where\">where\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/acpi/tables/struct.RawMADTEntryLocalApic.html\" title=\"struct svsm::acpi::tables::RawMADTEntryLocalApic\">RawMADTEntryLocalApic</a><div class=\"where\">where\n    <a class=\"struct\" href=\"svsm/acpi/tables/struct.RawMADTEntryHeader.html\" title=\"struct svsm::acpi::tables::RawMADTEntryHeader\">RawMADTEntryHeader</a>: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/acpi/tables/struct.RawMADTEntryLocalX2Apic.html\" title=\"struct svsm::acpi::tables::RawMADTEntryLocalX2Apic\">RawMADTEntryLocalX2Apic</a><div class=\"where\">where\n    <a class=\"struct\" href=\"svsm/acpi/tables/struct.RawMADTEntryHeader.html\" title=\"struct svsm::acpi::tables::RawMADTEntryHeader\">RawMADTEntryHeader</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">2</a>]: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/greq/msg/struct.SnpGuestRequestMsg.html\" title=\"struct svsm::greq::msg::SnpGuestRequestMsg\">SnpGuestRequestMsg</a><div class=\"where\">where\n    <a class=\"struct\" href=\"svsm/greq/msg/struct.SnpGuestRequestMsgHdr.html\" title=\"struct svsm::greq::msg::SnpGuestRequestMsgHdr\">SnpGuestRequestMsgHdr</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">4000</a>]: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/greq/msg/struct.SnpGuestRequestMsgHdr.html\" title=\"struct svsm::greq::msg::SnpGuestRequestMsgHdr\">SnpGuestRequestMsgHdr</a><div class=\"where\">where\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">32</a>]: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u64.html\">u64</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">8</a>]: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u16.html\">u16</a>: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">35</a>]: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/greq/pld_report/struct.AttestationReport.html\" title=\"struct svsm::greq::pld_report::AttestationReport\">AttestationReport</a><div class=\"where\">where\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u64.html\">u64</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">16</a>]: FromBytes,\n    <a class=\"struct\" href=\"svsm/greq/pld_report/struct.TcbVersion.html\" title=\"struct svsm::greq::pld_report::TcbVersion\">TcbVersion</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">64</a>]: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">48</a>]: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">32</a>]: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">24</a>]: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">192</a>]: FromBytes,\n    <a class=\"struct\" href=\"svsm/greq/pld_report/struct.Signature.html\" title=\"struct svsm::greq::pld_report::Signature\">Signature</a>: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.unit.html\">()</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/greq/pld_report/struct.Signature.html\" title=\"struct svsm::greq::pld_report::Signature\">Signature</a><div class=\"where\">where\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">72</a>]: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">368</a>]: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/greq/pld_report/struct.SnpReportRequest.html\" title=\"struct svsm::greq::pld_report::SnpReportRequest\">SnpReportRequest</a><div class=\"where\">where\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">64</a>]: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">24</a>]: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/greq/pld_report/struct.SnpReportResponse.html\" title=\"struct svsm::greq::pld_report::SnpReportResponse\">SnpReportResponse</a><div class=\"where\">where\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">24</a>]: FromBytes,\n    <a class=\"struct\" href=\"svsm/greq/pld_report/struct.AttestationReport.html\" title=\"struct svsm::greq::pld_report::AttestationReport\">AttestationReport</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/greq/pld_report/struct.TcbVersion.html\" title=\"struct svsm::greq::pld_report::TcbVersion\">TcbVersion</a><div class=\"where\">where\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u64.html\">u64</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/platform/snp_fw/struct.SevMetaDataDesc.html\" title=\"struct svsm::platform::snp_fw::SevMetaDataDesc\">SevMetaDataDesc</a><div class=\"where\">where\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/platform/snp_fw/struct.SevMetaDataHeader.html\" title=\"struct svsm::platform::snp_fw::SevMetaDataHeader\">SevMetaDataHeader</a><div class=\"where\">where\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">4</a>]: FromBytes,\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u32.html\">u32</a>: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/utils/fw_meta/struct.RawMetaBuffer.html\" title=\"struct svsm::utils::fw_meta::RawMetaBuffer\">RawMetaBuffer</a><div class=\"where\">where\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">4046</a>]: FromBytes,\n    <a class=\"struct\" href=\"svsm/utils/fw_meta/struct.RawMetaHeader.html\" title=\"struct svsm::utils::fw_meta::RawMetaHeader\">RawMetaHeader</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">32</a>]: FromBytes,</div>"],["impl FromBytes for <a class=\"struct\" href=\"svsm/utils/fw_meta/struct.RawMetaHeader.html\" title=\"struct svsm::utils::fw_meta::RawMetaHeader\">RawMetaHeader</a><div class=\"where\">where\n    <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u16.html\">u16</a>: FromBytes,\n    [<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.u8.html\">u8</a>; <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.86.0/core/primitive.array.html\">16</a>]: FromBytes,</div>"]]]]);
    if (window.register_implementors) {
        window.register_implementors(implementors);
    } else {
        window.pending_implementors = implementors;
    }
})()
//{"start":57,"fragment_lengths":[12216]}